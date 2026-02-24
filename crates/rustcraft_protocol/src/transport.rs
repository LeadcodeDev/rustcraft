use std::io::{self, Read, Write};
use std::sync::{Mutex, mpsc};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::protocol::{ClientMessage, ServerMessage};

/// Transport trait for the client side (sends ClientMessage, receives ServerMessage).
pub trait ClientTransport: Send + Sync + 'static {
    fn send(&self, msg: ClientMessage);
    fn receive(&self) -> Vec<ServerMessage>;
}

/// Transport trait for the server side (sends ServerMessage, receives ClientMessage).
pub trait ServerTransport: Send + Sync + 'static {
    fn send(&self, client_id: u64, msg: ServerMessage);
    fn broadcast(&self, msg: ServerMessage);
    fn broadcast_except(&self, exclude_id: u64, msg: ServerMessage);
    fn receive(&self) -> Vec<(u64, ClientMessage)>;
    fn disconnect(&self, client_id: u64);
}

// --- Serialization helpers (length-prefixed bincode framing) ---

/// Write a length-prefixed, zlib-compressed bincode message to a writer.
pub fn write_message<W: Write, T: serde::Serialize>(writer: &mut W, msg: &T) -> io::Result<()> {
    let data =
        bincode::serialize(msg).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&data)?;
    let compressed = encoder.finish()?;
    let len = (compressed.len() as u32).to_be_bytes();
    writer.write_all(&len)?;
    writer.write_all(&compressed)?;
    writer.flush()?;
    Ok(())
}

/// Read a length-prefixed, zlib-compressed bincode message from a reader.
pub fn read_message<R: Read, T: serde::de::DeserializeOwned>(reader: &mut R) -> io::Result<T> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut compressed = vec![0u8; len];
    reader.read_exact(&mut compressed)?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data)?;
    bincode::deserialize(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

// --- Local transport (same-process via mpsc channels) ---

/// Local client transport using mpsc channels (same-process communication).
pub struct LocalClientTransport {
    tx: mpsc::Sender<ClientMessage>,
    rx: Mutex<mpsc::Receiver<ServerMessage>>,
}

impl ClientTransport for LocalClientTransport {
    fn send(&self, msg: ClientMessage) {
        let _ = self.tx.send(msg);
    }

    fn receive(&self) -> Vec<ServerMessage> {
        let rx = self.rx.lock().unwrap();
        let mut messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            messages.push(msg);
        }
        messages
    }
}

/// Local server transport using mpsc channels (same-process communication).
pub struct LocalServerTransport {
    tx: mpsc::Sender<ServerMessage>,
    rx: Mutex<mpsc::Receiver<ClientMessage>>,
}

impl ServerTransport for LocalServerTransport {
    fn send(&self, _client_id: u64, msg: ServerMessage) {
        let _ = self.tx.send(msg);
    }

    fn broadcast(&self, msg: ServerMessage) {
        let _ = self.tx.send(msg);
    }

    fn broadcast_except(&self, exclude_id: u64, msg: ServerMessage) {
        // In local mode, the only client is id 0. Skip if excluded.
        if exclude_id != 0 {
            let _ = self.tx.send(msg);
        }
    }

    fn receive(&self) -> Vec<(u64, ClientMessage)> {
        let rx = self.rx.lock().unwrap();
        let mut messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            // In local mode, there's only one client with id 0
            messages.push((0, msg));
        }
        messages
    }

    fn disconnect(&self, _client_id: u64) {
        // No-op for local transport
    }
}

/// Create a pair of local transports connected by mpsc channels.
/// Used for solo play (client and server in the same process).
pub fn create_local_transport() -> (LocalClientTransport, LocalServerTransport) {
    let (client_tx, server_rx) = mpsc::channel();
    let (server_tx, client_rx) = mpsc::channel();
    (
        LocalClientTransport {
            tx: client_tx,
            rx: Mutex::new(client_rx),
        },
        LocalServerTransport {
            tx: server_tx,
            rx: Mutex::new(server_rx),
        },
    )
}

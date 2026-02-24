use std::sync::{Mutex, mpsc};

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
    fn receive(&self) -> Vec<(u64, ClientMessage)>;
}

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

    fn receive(&self) -> Vec<(u64, ClientMessage)> {
        let rx = self.rx.lock().unwrap();
        let mut messages = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            // In local mode, there's only one client with id 0
            messages.push((0, msg));
        }
        messages
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

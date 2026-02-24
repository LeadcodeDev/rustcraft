use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::{ClientTransport, ServerTransport, read_message, write_message};

// --- TCP Server Transport ---

struct TcpClient {
    writer: BufWriter<TcpStream>,
}

pub struct TcpServerTransport {
    /// Incoming messages from all clients: (client_id, ClientMessage)
    incoming: Arc<Mutex<Vec<(u64, ClientMessage)>>>,
    /// Connected clients (writer half)
    clients: Arc<Mutex<HashMap<u64, TcpClient>>>,
}

impl TcpServerTransport {
    pub fn new(addr: impl ToSocketAddrs) -> Self {
        let listener = TcpListener::bind(addr).expect("Failed to bind TCP listener");
        listener
            .set_nonblocking(false)
            .expect("Failed to set listener blocking");

        let incoming: Arc<Mutex<Vec<(u64, ClientMessage)>>> = Arc::new(Mutex::new(Vec::new()));
        let clients: Arc<Mutex<HashMap<u64, TcpClient>>> = Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(0));

        // Spawn acceptor thread
        let incoming_clone = Arc::clone(&incoming);
        let clients_clone = Arc::clone(&clients);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };

                let client_id = next_id.fetch_add(1, Ordering::SeqCst);

                // Clone the stream for the reader thread
                let read_stream = match stream.try_clone() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // Store the writer
                {
                    let mut clients_lock = clients_clone.lock().unwrap();
                    clients_lock.insert(
                        client_id,
                        TcpClient {
                            writer: BufWriter::new(stream),
                        },
                    );
                }

                // Spawn reader thread for this client
                let incoming_for_reader = Arc::clone(&incoming_clone);
                let clients_for_reader = Arc::clone(&clients_clone);

                thread::spawn(move || {
                    let mut reader = BufReader::new(read_stream);
                    loop {
                        match read_message::<_, ClientMessage>(&mut reader) {
                            Ok(msg) => {
                                incoming_for_reader.lock().unwrap().push((client_id, msg));
                            }
                            Err(_) => {
                                // Connection closed or error — push a Disconnect message
                                incoming_for_reader
                                    .lock()
                                    .unwrap()
                                    .push((client_id, ClientMessage::Disconnect));
                                // Remove client from writers
                                clients_for_reader.lock().unwrap().remove(&client_id);
                                break;
                            }
                        }
                    }
                });
            }
        });

        Self {
            incoming,
            clients,
        }
    }
}

impl ServerTransport for TcpServerTransport {
    fn send(&self, client_id: u64, msg: ServerMessage) {
        let mut clients = self.clients.lock().unwrap();
        if let Some(client) = clients.get_mut(&client_id) {
            if write_message(&mut client.writer, &msg).is_err() {
                clients.remove(&client_id);
            }
        }
    }

    fn broadcast(&self, msg: ServerMessage) {
        let mut clients = self.clients.lock().unwrap();
        let mut disconnected = Vec::new();
        for (&id, client) in clients.iter_mut() {
            if write_message(&mut client.writer, &msg).is_err() {
                disconnected.push(id);
            }
        }
        for id in disconnected {
            clients.remove(&id);
        }
    }

    fn broadcast_except(&self, exclude_id: u64, msg: ServerMessage) {
        let mut clients = self.clients.lock().unwrap();
        let mut disconnected = Vec::new();
        for (&id, client) in clients.iter_mut() {
            if id == exclude_id {
                continue;
            }
            if write_message(&mut client.writer, &msg).is_err() {
                disconnected.push(id);
            }
        }
        for id in disconnected {
            clients.remove(&id);
        }
    }

    fn receive(&self) -> Vec<(u64, ClientMessage)> {
        let mut incoming = self.incoming.lock().unwrap();
        std::mem::take(&mut *incoming)
    }

    fn disconnect(&self, client_id: u64) {
        self.clients.lock().unwrap().remove(&client_id);
    }
}

// --- TCP Client Transport ---

pub struct TcpClientTransport {
    writer: Mutex<BufWriter<TcpStream>>,
    incoming: Arc<Mutex<Vec<ServerMessage>>>,
}

impl TcpClientTransport {
    pub fn connect(addr: impl ToSocketAddrs) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let read_stream = stream.try_clone()?;

        let incoming: Arc<Mutex<Vec<ServerMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let incoming_clone = Arc::clone(&incoming);

        // Spawn reader thread
        thread::spawn(move || {
            let mut reader = BufReader::new(read_stream);
            loop {
                match read_message::<_, ServerMessage>(&mut reader) {
                    Ok(msg) => {
                        incoming_clone.lock().unwrap().push(msg);
                    }
                    Err(_) => {
                        // Connection lost
                        break;
                    }
                }
            }
        });

        Ok(Self {
            writer: Mutex::new(BufWriter::new(stream)),
            incoming,
        })
    }
}

impl ClientTransport for TcpClientTransport {
    fn send(&self, msg: ClientMessage) {
        let mut writer = self.writer.lock().unwrap();
        let _ = write_message(&mut *writer, &msg);
    }

    fn receive(&self) -> Vec<ServerMessage> {
        let mut incoming = self.incoming.lock().unwrap();
        std::mem::take(&mut *incoming)
    }
}

pub mod systems;
pub mod world_session;

use std::sync::Mutex;

use bevy::prelude::*;

use rustcraft_protocol::protocol::ServerMessage;
use rustcraft_protocol::transport::ServerTransport;

use systems::{
    ServerTransportRes, server_dropped_item_physics, server_pickup_items,
    server_process_messages,
};
use world_session::WorldSession;

pub struct ServerPlugin {
    transport: Mutex<Option<Box<dyn ServerTransport>>>,
    world_name: String,
    seed: u32,
}

impl ServerPlugin {
    pub fn new(transport: impl ServerTransport, world_name: impl Into<String>, seed: u32) -> Self {
        Self {
            transport: Mutex::new(Some(Box::new(transport))),
            world_name: world_name.into(),
            seed,
        }
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let transport = self
            .transport
            .lock()
            .unwrap()
            .take()
            .expect("ServerPlugin transport already taken");

        let session = WorldSession::new(self.world_name.clone(), self.seed);

        app.insert_resource(ServerTransportRes(transport))
            .insert_resource(session)
            .add_systems(Startup, send_initial_chunk_data)
            .add_systems(
                Update,
                (
                    server_process_messages,
                    server_dropped_item_physics.after(server_process_messages),
                    server_pickup_items.after(server_dropped_item_physics),
                ),
            );
    }
}

/// Sends all chunk data to connected clients at startup.
fn send_initial_chunk_data(session: Res<WorldSession>, transport: Res<ServerTransportRes>) {
    for (&pos, chunk) in &session.chunk_map.chunks {
        transport.0.broadcast(ServerMessage::ChunkData {
            pos: (pos.0, pos.1),
            blocks: chunk.blocks.clone(),
        });
    }
}

use bevy::prelude::*;

use rustcraft_protocol::chunk::{Chunk, ChunkPos};
use rustcraft_protocol::protocol::ServerMessage;

use crate::ClientTransportRes;
use crate::world::chunk::ChunkMap;

/// Receives all server messages and applies them to the client state.
pub fn client_receive_messages(
    transport: Res<ClientTransportRes>,
    mut chunk_map: ResMut<ChunkMap>,
) {
    let messages = transport.0.receive();

    for msg in messages {
        match msg {
            ServerMessage::ChunkData { pos, blocks } => {
                let chunk_pos = ChunkPos(pos.0, pos.1);
                let mut chunk = Chunk::new();
                chunk.blocks = blocks;
                chunk_map.chunks.insert(chunk_pos, chunk);
            }
            ServerMessage::BlockChanged { position, new_type } => {
                chunk_map.set_block(position.x, position.y, position.z, new_type);
            }
            // Other messages will be handled as we implement prediction/reconciliation
            _ => {}
        }
    }
}

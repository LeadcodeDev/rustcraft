use std::collections::HashMap;

use bevy::prelude::*;

use rustcraft_protocol::block::BlockType;
use rustcraft_protocol::chunk::{Chunk, ChunkPos};
use rustcraft_protocol::protocol::ServerMessage;

use crate::ClientTransportRes;
use crate::LocalPlayerId;
use crate::events::{
    GameModeChangedEvent, PlayerJoinEvent, PlayerLeaveEvent,
};
use crate::inventory::Inventory;
use crate::player::camera::{FlyCam, GameMode, Player};
use crate::world::chunk::ChunkMap;

/// Stores the target positions for remote players (for interpolation).
#[derive(Resource, Default)]
pub struct RemotePlayerStates {
    pub players: HashMap<u64, RemotePlayerTarget>,
}

pub struct RemotePlayerTarget {
    pub name: String,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

/// Event fired when the server spawns a dropped item.
#[derive(Event)]
pub struct ServerDroppedItemSpawnEvent {
    pub id: u64,
    pub block: BlockType,
    pub count: u32,
    pub position: Vec3,
}

/// Event fired when the server removes a dropped item.
#[derive(Event)]
pub struct ServerDroppedItemRemoveEvent {
    pub id: u64,
}

/// Receives all server messages and applies them to the client state.
#[allow(clippy::too_many_arguments)]
pub fn client_receive_messages(
    transport: Res<ClientTransportRes>,
    mut local_player_id: ResMut<LocalPlayerId>,
    mut chunk_map: ResMut<ChunkMap>,
    mut inventory: ResMut<Inventory>,
    mut game_mode: ResMut<GameMode>,
    mut remote_players: ResMut<RemotePlayerStates>,
    mut ev_player_join: EventWriter<PlayerJoinEvent>,
    mut ev_player_leave: EventWriter<PlayerLeaveEvent>,
    mut ev_gamemode_changed: EventWriter<GameModeChangedEvent>,
    mut ev_item_spawn: EventWriter<ServerDroppedItemSpawnEvent>,
    mut ev_item_remove: EventWriter<ServerDroppedItemRemoveEvent>,
    mut query: Query<(&mut Transform, &mut Player), With<FlyCam>>,
) {
    let messages = transport.0.receive();

    for msg in messages {
        match msg {
            ServerMessage::ConnectAccepted { player_id } => {
                local_player_id.0 = Some(player_id);
                info!("Connected to server as player {}", player_id);
            }

            ServerMessage::ConnectRejected { reason } => {
                error!("Connection rejected: {}", reason);
            }

            ServerMessage::PlayerJoined {
                player_id,
                name,
                position,
            } => {
                // Don't add ourselves as a remote player
                if Some(player_id) == local_player_id.0 {
                    continue;
                }
                remote_players.players.insert(
                    player_id,
                    RemotePlayerTarget {
                        name: name.clone(),
                        position,
                        yaw: 0.0,
                        pitch: 0.0,
                    },
                );
                ev_player_join.send(PlayerJoinEvent {
                    player_id,
                    name,
                    position,
                });
            }

            ServerMessage::PlayerLeft { player_id } => {
                remote_players.players.remove(&player_id);
                ev_player_leave.send(PlayerLeaveEvent { player_id });
            }

            ServerMessage::PlayerPositionUpdate {
                player_id,
                position,
                yaw,
                pitch,
            } => {
                if let Some(target) = remote_players.players.get_mut(&player_id) {
                    target.position = position;
                    target.yaw = yaw;
                    target.pitch = pitch;
                }
            }

            ServerMessage::PlayerStateUpdate {
                last_processed_input: _,
                position,
                velocity_y,
                grounded,
            } => {
                // Reconcile local player with server state.
                // Only snap if the server position diverges significantly
                // from local prediction to avoid frame-to-frame flickering.
                const RECONCILE_THRESHOLD: f32 = 0.1;

                for (mut transform, mut player) in &mut query {
                    let distance = player.position.distance(position);
                    if distance > RECONCILE_THRESHOLD {
                        player.position = position;
                        transform.translation =
                            position + Vec3::new(0.0, crate::player::camera::EYE_HEIGHT, 0.0);
                    }
                    player.velocity_y = velocity_y;
                    player.grounded = grounded;
                }
            }

            ServerMessage::ChunkData { pos, blocks } => {
                let chunk_pos = ChunkPos(pos.0, pos.1);
                let mut chunk = Chunk::new();
                chunk.blocks = blocks;
                chunk_map.chunks.insert(chunk_pos, chunk);
            }

            ServerMessage::ChunkUnload { pos } => {
                let chunk_pos = ChunkPos(pos.0, pos.1);
                chunk_map.chunks.remove(&chunk_pos);
            }

            ServerMessage::BlockChanged { position, new_type } => {
                chunk_map.set_block(position.x, position.y, position.z, new_type);
            }

            ServerMessage::InventoryUpdate { slots, active_slot } => {
                for (i, slot) in slots.into_iter().enumerate() {
                    if i < inventory.slots.len() {
                        inventory.slots[i] = slot;
                    }
                }
                inventory.active_slot = active_slot;
            }

            ServerMessage::GameModeChanged { mode } => {
                let new_mode = match mode {
                    rustcraft_protocol::game_mode::GameMode::Creative => GameMode::Creative,
                    rustcraft_protocol::game_mode::GameMode::Survival => GameMode::Survival,
                };
                *game_mode = new_mode;

                // Reset player velocity on game mode change
                for (_transform, mut player) in &mut query {
                    player.velocity_y = 0.0;
                }

                // Fire event for logging plugins
                if let Ok((transform, player)) = query.get_single() {
                    ev_gamemode_changed.send(GameModeChangedEvent {
                        new_mode,
                        player: player.location(&transform),
                    });
                }
            }

            ServerMessage::DroppedItemSpawned {
                id,
                stack,
                position,
                velocity: _,
            } => {
                ev_item_spawn.send(ServerDroppedItemSpawnEvent {
                    id,
                    block: stack.block,
                    count: stack.count,
                    position,
                });
            }

            ServerMessage::DroppedItemRemoved { id } => {
                ev_item_remove.send(ServerDroppedItemRemoveEvent { id });
            }
        }
    }
}

use bevy::prelude::*;

use rustcraft_protocol::block::BlockType;
use rustcraft_protocol::game_mode::GameMode;
use rustcraft_protocol::inventory::ItemStack;
use rustcraft_protocol::physics::{
    GRAVITY, JUMP_VELOCITY, TERMINAL_VELOCITY, is_on_ground, move_with_collision,
};
use rustcraft_protocol::protocol::{BlockAction, ClientMessage, ServerMessage};
use rustcraft_protocol::raycast::dda_raycast;
use rustcraft_protocol::transport::ServerTransport;

use crate::world_session::{DroppedItemState, WorldSession};

/// Bevy Resource wrapping a boxed ServerTransport.
#[derive(Resource)]
pub struct ServerTransportRes(pub Box<dyn ServerTransport>);

/// Process all incoming client messages and produce authoritative responses.
pub fn server_process_messages(
    mut session: ResMut<WorldSession>,
    transport: Res<ServerTransportRes>,
) {
    session.tick += 1;

    let messages = transport.0.receive();

    for (client_id, msg) in messages {
        // Destructure session for independent field access
        let WorldSession {
            ref mut chunk_map,
            ref mut players,
            ref mut inventories,
            ref mut dropped_items,
            ref mut next_entity_id,
            ..
        } = *session;

        match msg {
            ClientMessage::InputCommand {
                sequence,
                dt,
                yaw,
                pitch,
                forward,
                backward,
                left,
                right,
                jump,
                sneak,
            } => {
                let Some(player) = players.get_mut(&client_id) else {
                    continue;
                };

                player.yaw = yaw;
                player.pitch = pitch;

                let speed = 12.0;

                let fwd = Vec3::new(
                    -yaw.sin() * pitch.cos(),
                    -pitch.sin(),
                    -yaw.cos() * pitch.cos(),
                )
                .normalize_or_zero();
                let rgt = Vec3::new(yaw.cos(), 0.0, -yaw.sin()).normalize_or_zero();

                let delta = match player.game_mode {
                    GameMode::Creative => {
                        let mut velocity = Vec3::ZERO;
                        if forward {
                            velocity += fwd;
                        }
                        if backward {
                            velocity -= fwd;
                        }
                        if right {
                            velocity += rgt;
                        }
                        if left {
                            velocity -= rgt;
                        }
                        if jump {
                            velocity += Vec3::Y;
                        }
                        if sneak {
                            velocity -= Vec3::Y;
                        }
                        if velocity != Vec3::ZERO {
                            velocity = velocity.normalize();
                        }
                        velocity * speed * dt
                    }
                    GameMode::Survival => {
                        let fwd_xz = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
                        let rgt_xz = Vec3::new(rgt.x, 0.0, rgt.z).normalize_or_zero();

                        let mut horizontal = Vec3::ZERO;
                        if forward {
                            horizontal += fwd_xz;
                        }
                        if backward {
                            horizontal -= fwd_xz;
                        }
                        if right {
                            horizontal += rgt_xz;
                        }
                        if left {
                            horizontal -= rgt_xz;
                        }
                        if horizontal != Vec3::ZERO {
                            horizontal = horizontal.normalize();
                        }

                        player.grounded = is_on_ground(player.position, chunk_map);

                        if jump && player.grounded {
                            player.velocity_y = JUMP_VELOCITY;
                            player.grounded = false;
                        }

                        player.velocity_y -= GRAVITY * dt;
                        player.velocity_y = player.velocity_y.max(-TERMINAL_VELOCITY);

                        Vec3::new(
                            horizontal.x * speed * dt,
                            player.velocity_y * dt,
                            horizontal.z * speed * dt,
                        )
                    }
                };

                let (new_pos, hit_floor, hit_ceiling) =
                    move_with_collision(player.position, delta, chunk_map);
                player.position = new_pos;

                if player.game_mode == GameMode::Survival {
                    if hit_floor {
                        player.velocity_y = 0.0;
                        player.grounded = true;
                    }
                    if hit_ceiling {
                        player.velocity_y = 0.0;
                    }
                }

                transport.0.send(
                    client_id,
                    ServerMessage::PlayerStateUpdate {
                        last_processed_input: sequence,
                        position: player.position,
                        velocity_y: player.velocity_y,
                        grounded: player.grounded,
                    },
                );
            }

            ClientMessage::BlockInteraction {
                action,
                origin,
                direction,
            } => {
                let Some(hit) = dda_raycast(origin, direction, chunk_map) else {
                    continue;
                };

                // Copy game_mode before mutable borrow
                let game_mode = players
                    .get(&client_id)
                    .map(|p| p.game_mode)
                    .unwrap_or(GameMode::Creative);

                match action {
                    BlockAction::Break => {
                        let old_block = chunk_map.get_block(
                            hit.block_pos.x,
                            hit.block_pos.y,
                            hit.block_pos.z,
                        );
                        chunk_map.set_block(
                            hit.block_pos.x,
                            hit.block_pos.y,
                            hit.block_pos.z,
                            BlockType::Air,
                        );
                        transport.0.broadcast(ServerMessage::BlockChanged {
                            position: hit.block_pos,
                            new_type: BlockType::Air,
                        });

                        if game_mode == GameMode::Survival {
                            let block_center = Vec3::new(
                                hit.block_pos.x as f32 + 0.5,
                                hit.block_pos.y as f32 + 0.5,
                                hit.block_pos.z as f32 + 0.5,
                            );
                            let entity_id = *next_entity_id;
                            *next_entity_id += 1;
                            dropped_items.insert(
                                entity_id,
                                DroppedItemState {
                                    stack: ItemStack::new(old_block, 1),
                                    position: block_center,
                                    velocity: Vec3::new(0.0, 4.0, 0.0),
                                    grounded: false,
                                    age: 0.0,
                                },
                            );
                            transport.0.broadcast(ServerMessage::DroppedItemSpawned {
                                id: entity_id,
                                stack: ItemStack::new(old_block, 1),
                                position: block_center,
                                velocity: Vec3::new(0.0, 4.0, 0.0),
                            });
                        }
                    }
                    BlockAction::Place => {
                        let Some(inv) = inventories.get_mut(&client_id) else {
                            continue;
                        };
                        let Some(block) = inv.active_block() else {
                            continue;
                        };

                        let place_pos = hit.block_pos + hit.normal;
                        chunk_map.set_block(place_pos.x, place_pos.y, place_pos.z, block);

                        if game_mode == GameMode::Survival {
                            inv.consume_active();
                            transport.0.send(
                                client_id,
                                ServerMessage::InventoryUpdate {
                                    slots: inv.slots.to_vec(),
                                    active_slot: inv.active_slot,
                                },
                            );
                        }

                        transport.0.broadcast(ServerMessage::BlockChanged {
                            position: place_pos,
                            new_type: block,
                        });
                    }
                }
            }

            ClientMessage::DropItem {
                slot,
                count,
                direction,
            } => {
                // Copy player position before getting mutable inventory
                let player_pos = match players.get(&client_id) {
                    Some(p) => p.position,
                    None => continue,
                };

                let Some(inv) = inventories.get_mut(&client_id) else {
                    continue;
                };

                let Some(stack) = inv.slots[slot] else {
                    continue;
                };

                let drop_count = count.min(stack.count);
                if drop_count == 0 {
                    continue;
                }

                let fwd_xz =
                    Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
                let drop_pos = player_pos + Vec3::Y * 1.7 + fwd_xz * 0.5;
                let drop_velocity = direction * 3.0 + Vec3::Y * 2.0;

                // Remove from inventory
                if drop_count >= stack.count {
                    inv.slots[slot] = None;
                } else {
                    inv.slots[slot].as_mut().unwrap().count -= drop_count;
                }

                let inv_slots = inv.slots.to_vec();
                let inv_active = inv.active_slot;

                let entity_id = *next_entity_id;
                *next_entity_id += 1;
                dropped_items.insert(
                    entity_id,
                    DroppedItemState {
                        stack: ItemStack::new(stack.block, drop_count),
                        position: drop_pos,
                        velocity: drop_velocity,
                        grounded: false,
                        age: 0.0,
                    },
                );

                transport.0.broadcast(ServerMessage::DroppedItemSpawned {
                    id: entity_id,
                    stack: ItemStack::new(stack.block, drop_count),
                    position: drop_pos,
                    velocity: drop_velocity,
                });

                transport.0.send(
                    client_id,
                    ServerMessage::InventoryUpdate {
                        slots: inv_slots,
                        active_slot: inv_active,
                    },
                );
            }

            ClientMessage::ToggleGameMode => {
                let Some(player) = players.get_mut(&client_id) else {
                    continue;
                };

                player.game_mode = match player.game_mode {
                    GameMode::Creative => GameMode::Survival,
                    GameMode::Survival => GameMode::Creative,
                };
                player.velocity_y = 0.0;

                transport.0.send(
                    client_id,
                    ServerMessage::GameModeChanged {
                        mode: player.game_mode,
                    },
                );
            }
        }
    }
}

/// Simulate dropped item physics on the server.
pub fn server_dropped_item_physics(time: Res<Time>, mut session: ResMut<WorldSession>) {
    let dt = time.delta_secs();
    let gravity = 32.0_f32;
    let terminal_velocity = 50.0_f32;
    let dropped_item_scale = 0.3_f32;

    let WorldSession {
        ref chunk_map,
        ref mut dropped_items,
        ..
    } = *session;

    for (_id, item) in dropped_items.iter_mut() {
        item.age += dt;

        if item.grounded {
            item.velocity.x *= (1.0 - 5.0 * dt).max(0.0);
            item.velocity.z *= (1.0 - 5.0 * dt).max(0.0);
            if item.velocity.x.abs() < 0.01 && item.velocity.z.abs() < 0.01 {
                item.velocity = Vec3::ZERO;
                continue;
            }

            let new_x = item.position.x + item.velocity.x * dt;
            let by = (item.position.y - 0.01).floor() as i32;
            let bz = item.position.z.floor() as i32;
            if !chunk_map
                .get_block(new_x.floor() as i32, by + 1, bz)
                .is_solid()
            {
                item.position.x = new_x;
            } else {
                item.velocity.x = 0.0;
            }

            let new_z = item.position.z + item.velocity.z * dt;
            let bx = item.position.x.floor() as i32;
            if !chunk_map
                .get_block(bx, by + 1, new_z.floor() as i32)
                .is_solid()
            {
                item.position.z = new_z;
            } else {
                item.velocity.z = 0.0;
            }
            continue;
        }

        // Airborne
        item.velocity.y -= gravity * dt;
        item.velocity.y = item.velocity.y.max(-terminal_velocity);

        let new_x = item.position.x + item.velocity.x * dt;
        let by = item.position.y.floor() as i32;
        let bz = item.position.z.floor() as i32;
        if chunk_map
            .get_block(new_x.floor() as i32, by, bz)
            .is_solid()
        {
            item.velocity.x = 0.0;
        } else {
            item.position.x = new_x;
        }

        let new_z = item.position.z + item.velocity.z * dt;
        let bx = item.position.x.floor() as i32;
        if chunk_map
            .get_block(bx, by, new_z.floor() as i32)
            .is_solid()
        {
            item.velocity.z = 0.0;
        } else {
            item.position.z = new_z;
        }

        let new_y = item.position.y + item.velocity.y * dt;
        let check_y = (new_y - 0.01).floor() as i32;
        let bx = item.position.x.floor() as i32;
        let bz = item.position.z.floor() as i32;

        if item.velocity.y <= 0.0 && chunk_map.get_block(bx, check_y, bz).is_solid() {
            item.position.y = (check_y + 1) as f32 + dropped_item_scale / 2.0 + 0.02;
            item.velocity.y = 0.0;
            item.grounded = true;
        } else {
            item.position.y = new_y;
        }
    }
}

/// Check for dropped item pickups based on player proximity.
pub fn server_pickup_items(mut session: ResMut<WorldSession>, transport: Res<ServerTransportRes>) {
    let pickup_radius = 2.0_f32;
    let pickup_delay = 1.5_f32;

    // Collect pickup candidates (no mutable borrow yet)
    let mut collected: Vec<(u64, u64)> = Vec::new();

    for (&entity_id, item) in session.dropped_items.iter() {
        if item.age < pickup_delay {
            continue;
        }

        for (&client_id, player) in session.players.iter() {
            let distance = player.position.distance(item.position);
            if distance <= pickup_radius {
                if let Some(inv) = session.inventories.get(&client_id) {
                    if inv.find_slot_for(item.stack.block).is_some() {
                        collected.push((entity_id, client_id));
                        break;
                    }
                }
            }
        }
    }

    // Now process collections with mutable access
    for (entity_id, client_id) in collected {
        let Some(item) = session.dropped_items.remove(&entity_id) else {
            continue;
        };

        let Some(inv) = session.inventories.get_mut(&client_id) else {
            continue;
        };

        inv.add_stack(item.stack.block, item.stack.count);

        let slots = inv.slots.to_vec();
        let active_slot = inv.active_slot;

        transport
            .0
            .broadcast(ServerMessage::DroppedItemRemoved { id: entity_id });
        transport.0.send(
            client_id,
            ServerMessage::InventoryUpdate {
                slots,
                active_slot,
            },
        );
    }
}

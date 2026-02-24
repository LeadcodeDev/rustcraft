use bevy::prelude::*;

use crate::ClientTransportRes;
use crate::inventory::Inventory;
use crate::player::camera::{FlyCam, GameMode, GameState, Player};
use crate::world::chunk::ChunkMap;

use rustcraft_protocol::block::BlockType;
use rustcraft_protocol::protocol::{BlockAction, ClientMessage};
use rustcraft_protocol::raycast::dda_raycast;

pub fn block_interaction(
    game_state: Res<GameState>,
    game_mode: Res<GameMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<FlyCam>>,
    transport: Res<ClientTransportRes>,
    mut chunk_map: ResMut<ChunkMap>,
    mut inventory: ResMut<Inventory>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    let left = mouse.just_pressed(MouseButton::Left);
    let right = mouse.just_pressed(MouseButton::Right);

    if !left && !right {
        return;
    }

    let Ok(cam_transform) = camera_query.get_single() else {
        return;
    };

    let origin = cam_transform.translation;
    let direction = cam_transform.forward().as_vec3();

    let action = if left {
        BlockAction::Break
    } else {
        BlockAction::Place
    };

    // Apply locally first (client-side prediction)
    if let Some(hit) = dda_raycast(origin, direction, &chunk_map.0) {
        match action {
            BlockAction::Break => {
                chunk_map.set_block(
                    hit.block_pos.x,
                    hit.block_pos.y,
                    hit.block_pos.z,
                    BlockType::Air,
                );
            }
            BlockAction::Place => {
                if let Some(block) = inventory.active_block() {
                    let place_pos = hit.block_pos + hit.normal;
                    chunk_map.set_block(place_pos.x, place_pos.y, place_pos.z, block);
                    if *game_mode == GameMode::Survival {
                        inventory.consume_active();
                    }
                }
            }
        }
    }

    // Then send to server for authoritative validation
    transport.0.send(ClientMessage::BlockInteraction {
        action,
        origin,
        direction,
    });
}

pub fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
        });
}

// --- Debug overlay ---

#[derive(Component)]
pub struct DebugOverlay;

#[derive(Component)]
pub struct DebugOverlayRoot;

#[derive(Resource)]
pub struct DebugOverlayVisible(pub bool);

impl Default for DebugOverlayVisible {
    fn default() -> Self {
        Self(false)
    }
}

pub fn spawn_debug_overlay(mut commands: Commands) {
    commands
        .spawn((
            DebugOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                DebugOverlay,
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn toggle_debug_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<DebugOverlayVisible>,
    mut query: Query<&mut Visibility, With<DebugOverlayRoot>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        visible.0 = !visible.0;
        for mut vis in &mut query {
            *vis = if visible.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub fn update_debug_overlay(
    visible: Res<DebugOverlayVisible>,
    game_mode: Res<GameMode>,
    camera_query: Query<(&Transform, &Player), With<FlyCam>>,
    mut text_query: Query<&mut Text, With<DebugOverlay>>,
) {
    if !visible.0 {
        return;
    }

    let Ok((transform, player)) = camera_query.get_single() else {
        return;
    };

    let pos = player.position;
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let yaw_deg = yaw.to_degrees();
    let pitch_deg = pitch.to_degrees();

    // Normalize yaw to 0..360
    let yaw_norm = ((yaw_deg % 360.0) + 360.0) % 360.0;
    let cardinal = match yaw_norm {
        y if y >= 315.0 || y < 45.0 => "South",
        y if y >= 45.0 && y < 135.0 => "West",
        y if y >= 135.0 && y < 225.0 => "North",
        _ => "East",
    };

    for mut text in &mut text_query {
        **text = format!(
            "XYZ: {:.1} / {:.1} / {:.1}\nFacing: {} ({:.1} / {:.1})\nGameMode: {:?}",
            pos.x, pos.y, pos.z, cardinal, yaw_deg, pitch_deg, *game_mode
        );
    }
}

const DROP_HOLD_DELAY: f32 = 0.5;
const DROP_REPEAT_INTERVAL: f32 = 0.05;

#[derive(Resource, Default)]
pub struct DropKeyState {
    held_time: f32,
    dropped_initial: bool,
}

pub fn drop_active_item(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut drop_state: ResMut<DropKeyState>,
    mut inventory: ResMut<Inventory>,
    camera_query: Query<&Transform, With<FlyCam>>,
    transport: Res<ClientTransportRes>,
) {
    if *game_state != GameState::Playing || !keys.pressed(KeyCode::KeyR) {
        drop_state.held_time = 0.0;
        drop_state.dropped_initial = false;
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Shift+R: drop entire stack at once, no repeat
    if shift {
        if !keys.just_pressed(KeyCode::KeyR) {
            return;
        }
    } else {
        let should_drop = if !drop_state.dropped_initial {
            drop_state.dropped_initial = true;
            true
        } else {
            drop_state.held_time += time.delta_secs();
            if drop_state.held_time >= DROP_HOLD_DELAY {
                drop_state.held_time -= DROP_REPEAT_INTERVAL;
                true
            } else {
                false
            }
        };

        if !should_drop {
            return;
        }
    }

    let Ok(transform) = camera_query.get_single() else {
        return;
    };

    let Some(stack) = inventory.slots[inventory.active_slot] else {
        return;
    };

    let drop_count = if shift { stack.count } else { 1 };
    let direction = transform.forward().as_vec3();

    // Apply locally first (client-side prediction)
    let slot = inventory.active_slot;
    if drop_count >= stack.count {
        inventory.slots[slot] = None;
    } else {
        inventory.slots[slot].as_mut().unwrap().count -= drop_count;
    }

    transport.0.send(ClientMessage::DropItem {
        slot,
        count: drop_count,
        direction,
    });
}

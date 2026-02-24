use bevy::prelude::*;

use crate::events::{BlockPlacedEvent, BlockRemovedEvent, ItemDroppedToWorldEvent};
use crate::inventory::Inventory;
use crate::player::camera::{FlyCam, GameMode, GameState, Player};
use crate::world::block::BlockType;
use crate::world::chunk::ChunkMap;

const MAX_REACH: f32 = 8.0;

struct RaycastHit {
    block_pos: IVec3,
    normal: IVec3,
}

fn dda_raycast(origin: Vec3, direction: Vec3, chunk_map: &ChunkMap) -> Option<RaycastHit> {
    let dir = direction.normalize();

    let mut pos = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );

    let step = IVec3::new(
        if dir.x >= 0.0 { 1 } else { -1 },
        if dir.y >= 0.0 { 1 } else { -1 },
        if dir.z >= 0.0 { 1 } else { -1 },
    );

    let t_delta = Vec3::new(
        if dir.x != 0.0 {
            (1.0 / dir.x).abs()
        } else {
            f32::MAX
        },
        if dir.y != 0.0 {
            (1.0 / dir.y).abs()
        } else {
            f32::MAX
        },
        if dir.z != 0.0 {
            (1.0 / dir.z).abs()
        } else {
            f32::MAX
        },
    );

    let mut t_max = Vec3::new(
        if dir.x >= 0.0 {
            ((pos.x as f32 + 1.0) - origin.x) / dir.x
        } else if dir.x != 0.0 {
            (pos.x as f32 - origin.x) / dir.x
        } else {
            f32::MAX
        },
        if dir.y >= 0.0 {
            ((pos.y as f32 + 1.0) - origin.y) / dir.y
        } else if dir.y != 0.0 {
            (pos.y as f32 - origin.y) / dir.y
        } else {
            f32::MAX
        },
        if dir.z >= 0.0 {
            ((pos.z as f32 + 1.0) - origin.z) / dir.z
        } else if dir.z != 0.0 {
            (pos.z as f32 - origin.z) / dir.z
        } else {
            f32::MAX
        },
    );

    let mut normal = IVec3::ZERO;
    let max_steps = (MAX_REACH * 3.0) as i32;

    for _ in 0..max_steps {
        let block = chunk_map.get_block(pos.x, pos.y, pos.z);
        if block.is_solid() {
            return Some(RaycastHit {
                block_pos: pos,
                normal,
            });
        }

        if t_max.x < t_max.y && t_max.x < t_max.z {
            pos.x += step.x;
            t_max.x += t_delta.x;
            normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y < t_max.z {
            pos.y += step.y;
            t_max.y += t_delta.y;
            normal = IVec3::new(0, -step.y, 0);
        } else {
            pos.z += step.z;
            t_max.z += t_delta.z;
            normal = IVec3::new(0, 0, -step.z);
        }

        let dist_sq =
            (Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32) - origin).length_squared();
        if dist_sq > MAX_REACH * MAX_REACH {
            return None;
        }
    }

    None
}

pub fn block_interaction(
    game_state: Res<GameState>,
    game_mode: Res<GameMode>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut chunk_map: ResMut<ChunkMap>,
    camera_query: Query<(&Transform, &Player), With<FlyCam>>,
    mut inventory: ResMut<Inventory>,
    mut ev_placed: EventWriter<BlockPlacedEvent>,
    mut ev_removed: EventWriter<BlockRemovedEvent>,
    mut ev_item_drop: EventWriter<ItemDroppedToWorldEvent>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    let left = mouse.just_pressed(MouseButton::Left);
    let right = mouse.just_pressed(MouseButton::Right);

    if !left && !right {
        return;
    }

    let Ok((cam_transform, player)) = camera_query.get_single() else {
        return;
    };

    let location = player.location(cam_transform);
    let origin = cam_transform.translation;
    let direction = cam_transform.forward().as_vec3();

    if let Some(hit) = dda_raycast(origin, direction, &chunk_map) {
        if left {
            let old_block = chunk_map.get_block(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z);
            chunk_map.set_block(
                hit.block_pos.x,
                hit.block_pos.y,
                hit.block_pos.z,
                BlockType::Air,
            );
            ev_removed.send(BlockRemovedEvent {
                position: hit.block_pos,
                block_type: old_block,
                player: location,
            });

            if *game_mode == GameMode::Survival {
                let block_center = Vec3::new(
                    hit.block_pos.x as f32 + 0.5,
                    hit.block_pos.y as f32 + 0.5,
                    hit.block_pos.z as f32 + 0.5,
                );
                ev_item_drop.send(ItemDroppedToWorldEvent {
                    block_type: old_block,
                    count: 1,
                    position: block_center,
                    velocity: Vec3::new(0.0, 4.0, 0.0),
                    player: location,
                });
            }
        } else if right {
            if let Some(block) = inventory.active_block() {
                let place_pos = hit.block_pos + hit.normal;
                chunk_map.set_block(place_pos.x, place_pos.y, place_pos.z, block);
                if *game_mode == GameMode::Survival {
                    inventory.consume_active();
                }
                ev_placed.send(BlockPlacedEvent {
                    position: place_pos,
                    block_type: block,
                    player: location,
                });
            }
        }
    }
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
    camera_query: Query<(&Transform, &Player), With<FlyCam>>,
    mut ev_item_drop: EventWriter<ItemDroppedToWorldEvent>,
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

    let Ok((transform, player)) = camera_query.get_single() else {
        return;
    };

    let Some(stack) = inventory.slots[inventory.active_slot] else {
        return;
    };

    let drop_count = if shift { stack.count } else { 1 };

    let forward = transform.forward().as_vec3();
    let drop_pos = player.position
        + Vec3::Y * 1.7
        + Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero() * 0.5;

    ev_item_drop.send(ItemDroppedToWorldEvent {
        block_type: stack.block,
        count: drop_count,
        position: drop_pos,
        velocity: forward * 3.0 + Vec3::Y * 2.0,
        player: player.location(transform),
    });

    if shift {
        let slot = inventory.active_slot;
        inventory.slots[slot] = None;
    } else {
        inventory.consume_active();
    }
}

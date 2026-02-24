use bevy_math::Vec3;

use crate::chunk::ChunkMap;
use crate::game_mode::GameMode;

pub const PLAYER_HALF_WIDTH: f32 = 0.3;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.7;
pub const GRAVITY: f32 = 32.0;
pub const JUMP_VELOCITY: f32 = 9.0;
pub const TERMINAL_VELOCITY: f32 = 78.4;

pub fn collides_with_world(pos: Vec3, chunk_map: &ChunkMap) -> bool {
    let min_x = (pos.x - PLAYER_HALF_WIDTH).floor() as i32;
    let max_x = (pos.x + PLAYER_HALF_WIDTH - 0.001).floor() as i32;
    let min_y = pos.y.floor() as i32;
    let max_y = (pos.y + PLAYER_HEIGHT - 0.001).floor() as i32;
    let min_z = (pos.z - PLAYER_HALF_WIDTH).floor() as i32;
    let max_z = (pos.z + PLAYER_HALF_WIDTH - 0.001).floor() as i32;

    for bx in min_x..=max_x {
        for by in min_y..=max_y {
            for bz in min_z..=max_z {
                if chunk_map.get_block(bx, by, bz).is_solid() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn is_on_ground(pos: Vec3, chunk_map: &ChunkMap) -> bool {
    let check_pos = Vec3::new(pos.x, pos.y - 0.001, pos.z);
    let min_x = (check_pos.x - PLAYER_HALF_WIDTH).floor() as i32;
    let max_x = (check_pos.x + PLAYER_HALF_WIDTH - 0.001).floor() as i32;
    let min_z = (check_pos.z - PLAYER_HALF_WIDTH).floor() as i32;
    let max_z = (check_pos.z + PLAYER_HALF_WIDTH - 0.001).floor() as i32;
    let by = (check_pos.y).floor() as i32;

    for bx in min_x..=max_x {
        for bz in min_z..=max_z {
            if chunk_map.get_block(bx, by, bz).is_solid() {
                return true;
            }
        }
    }
    false
}

pub fn move_with_collision(
    current_pos: Vec3,
    delta: Vec3,
    chunk_map: &ChunkMap,
) -> (Vec3, bool, bool) {
    let mut pos = current_pos;
    let mut hit_floor = false;
    let mut hit_ceiling = false;

    // X axis
    pos.x += delta.x;
    if collides_with_world(pos, chunk_map) {
        if delta.x > 0.0 {
            pos.x = (pos.x + PLAYER_HALF_WIDTH).floor() - PLAYER_HALF_WIDTH;
        } else {
            pos.x = (pos.x - PLAYER_HALF_WIDTH).floor() + 1.0 + PLAYER_HALF_WIDTH;
        }
    }

    // Y axis
    pos.y += delta.y;
    if collides_with_world(pos, chunk_map) {
        if delta.y > 0.0 {
            pos.y = (pos.y + PLAYER_HEIGHT).floor() - PLAYER_HEIGHT;
            hit_ceiling = true;
        } else {
            pos.y = pos.y.floor() + 1.0;
            hit_floor = true;
        }
    }

    // Z axis
    pos.z += delta.z;
    if collides_with_world(pos, chunk_map) {
        if delta.z > 0.0 {
            pos.z = (pos.z + PLAYER_HALF_WIDTH).floor() - PLAYER_HALF_WIDTH;
        } else {
            pos.z = (pos.z - PLAYER_HALF_WIDTH).floor() + 1.0 + PLAYER_HALF_WIDTH;
        }
    }

    (pos, hit_floor, hit_ceiling)
}

/// Input state for one frame, used by both client (prediction) and server (authoritative).
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub dt: f32,
}

/// Compute the movement delta from input state, player state, and game mode.
/// This is the shared movement logic used by both client and server.
pub fn compute_movement_delta(
    input: &InputState,
    player: &crate::player_state::PlayerState,
    chunk_map: &ChunkMap,
    game_mode: &GameMode,
) -> (Vec3, f32, bool) {
    let speed = 12.0;
    let yaw = input.yaw;
    let pitch = input.pitch;
    let dt = input.dt;

    // Compute forward/right vectors from yaw/pitch
    let forward = Vec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), -yaw.cos() * pitch.cos())
        .normalize_or_zero();
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin()).normalize_or_zero();

    let mut velocity_y = player.velocity_y;
    let mut grounded = player.grounded;

    let delta = match game_mode {
        GameMode::Creative => {
            let mut velocity = Vec3::ZERO;
            if input.forward {
                velocity += forward;
            }
            if input.backward {
                velocity -= forward;
            }
            if input.right {
                velocity += right;
            }
            if input.left {
                velocity -= right;
            }
            if input.jump {
                velocity += Vec3::Y;
            }
            if input.sneak {
                velocity -= Vec3::Y;
            }
            if velocity != Vec3::ZERO {
                velocity = velocity.normalize();
            }
            velocity * speed * dt
        }
        GameMode::Survival => {
            let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
            let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

            let mut horizontal = Vec3::ZERO;
            if input.forward {
                horizontal += forward_xz;
            }
            if input.backward {
                horizontal -= forward_xz;
            }
            if input.right {
                horizontal += right_xz;
            }
            if input.left {
                horizontal -= right_xz;
            }
            if horizontal != Vec3::ZERO {
                horizontal = horizontal.normalize();
            }

            grounded = is_on_ground(player.position, chunk_map);

            if input.jump && grounded {
                velocity_y = JUMP_VELOCITY;
                grounded = false;
            }

            velocity_y -= GRAVITY * dt;
            velocity_y = velocity_y.max(-TERMINAL_VELOCITY);

            Vec3::new(
                horizontal.x * speed * dt,
                velocity_y * dt,
                horizontal.z * speed * dt,
            )
        }
    };

    (delta, velocity_y, grounded)
}

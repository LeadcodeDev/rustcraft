use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::events::{GameModeChanged, PlayerMoved};
use crate::world::chunk::ChunkMap;

#[derive(Component)]
pub struct FlyCam;

#[derive(Component)]
pub struct Player {
    pub position: Vec3,
    pub velocity_y: f32,
    pub grounded: bool,
}

#[derive(Resource)]
pub struct CameraSettings {
    pub sensitivity: f32,
    pub speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.003,
            speed: 12.0,
        }
    }
}

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug)]
pub enum GameMode {
    Creative,
    Survival,
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::Creative
    }
}

#[derive(Resource, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
}

const PLAYER_HALF_WIDTH: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.8;
const EYE_HEIGHT: f32 = 1.7;
const GRAVITY: f32 = 32.0;
const JUMP_VELOCITY: f32 = 9.0;
const TERMINAL_VELOCITY: f32 = 78.4;

pub fn spawn_camera(mut commands: Commands) {
    let eye_pos = Vec3::new(64.0, 40.0, 64.0);
    let feet_pos = eye_pos - Vec3::new(0.0, EYE_HEIGHT, 0.0);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(eye_pos).looking_at(Vec3::new(64.0, 20.0, 0.0), Vec3::Y),
        FlyCam,
        Player {
            position: feet_pos,
            velocity_y: 0.0,
            grounded: false,
        },
    ));
}

pub fn initial_cursor_grab(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

pub fn camera_look(
    game_state: Res<GameState>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<CameraSettings>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    if *game_state == GameState::Paused {
        return;
    }
    if mouse_motion.delta == Vec2::ZERO {
        return;
    }

    for mut transform in &mut query {
        let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        yaw -= mouse_motion.delta.x * settings.sensitivity;
        pitch -= mouse_motion.delta.y * settings.sensitivity;
        pitch = pitch.clamp(-1.54, 1.54);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

fn collides_with_world(pos: Vec3, chunk_map: &ChunkMap) -> bool {
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

fn is_on_ground(pos: Vec3, chunk_map: &ChunkMap) -> bool {
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

fn move_with_collision(current_pos: Vec3, delta: Vec3, chunk_map: &ChunkMap) -> (Vec3, bool, bool) {
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

pub fn camera_movement(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    time: Res<Time>,
    chunk_map: Res<ChunkMap>,
    game_mode: Res<GameMode>,
    mut ev_moved: EventWriter<PlayerMoved>,
    mut query: Query<(&mut Transform, &mut Player), With<FlyCam>>,
) {
    if *game_state == GameState::Paused {
        return;
    }

    let dt = time.delta_secs();

    for (mut transform, mut player) in &mut query {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();

        let delta = match *game_mode {
            GameMode::Creative => {
                let mut velocity = Vec3::ZERO;
                if keys.pressed(KeyCode::KeyW) {
                    velocity += forward;
                }
                if keys.pressed(KeyCode::KeyS) {
                    velocity -= forward;
                }
                if keys.pressed(KeyCode::KeyD) {
                    velocity += right;
                }
                if keys.pressed(KeyCode::KeyA) {
                    velocity -= right;
                }
                if keys.pressed(KeyCode::Space) {
                    velocity += Vec3::Y;
                }
                if keys.pressed(KeyCode::ShiftLeft) {
                    velocity -= Vec3::Y;
                }
                if velocity != Vec3::ZERO {
                    velocity = velocity.normalize();
                }
                velocity * settings.speed * dt
            }
            GameMode::Survival => {
                let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

                let mut horizontal = Vec3::ZERO;
                if keys.pressed(KeyCode::KeyW) {
                    horizontal += forward_xz;
                }
                if keys.pressed(KeyCode::KeyS) {
                    horizontal -= forward_xz;
                }
                if keys.pressed(KeyCode::KeyD) {
                    horizontal += right_xz;
                }
                if keys.pressed(KeyCode::KeyA) {
                    horizontal -= right_xz;
                }
                if horizontal != Vec3::ZERO {
                    horizontal = horizontal.normalize();
                }

                player.grounded = is_on_ground(player.position, &chunk_map);

                if keys.just_pressed(KeyCode::Space) && player.grounded {
                    player.velocity_y = JUMP_VELOCITY;
                    player.grounded = false;
                }

                player.velocity_y -= GRAVITY * dt;
                player.velocity_y = player.velocity_y.max(-TERMINAL_VELOCITY);

                Vec3::new(
                    horizontal.x * settings.speed * dt,
                    player.velocity_y * dt,
                    horizontal.z * settings.speed * dt,
                )
            }
        };

        let old_pos = player.position;
        let (new_pos, hit_floor, hit_ceiling) =
            move_with_collision(player.position, delta, &chunk_map);
        player.position = new_pos;

        if *game_mode == GameMode::Survival {
            if hit_floor {
                player.velocity_y = 0.0;
                player.grounded = true;
            }
            if hit_ceiling {
                player.velocity_y = 0.0;
            }
        }

        if player.position != old_pos {
            ev_moved.send(PlayerMoved {
                old_position: old_pos,
                new_position: player.position,
            });
        }
        transform.translation = player.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
    }
}

pub fn toggle_gamemode(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game_mode: ResMut<GameMode>,
    mut ev_changed: EventWriter<GameModeChanged>,
    mut query: Query<&mut Player>,
) {
    if *game_state == GameState::Paused {
        return;
    }
    if keys.just_pressed(KeyCode::F1) {
        *game_mode = match *game_mode {
            GameMode::Creative => GameMode::Survival,
            GameMode::Survival => GameMode::Creative,
        };
        for mut player in &mut query {
            player.velocity_y = 0.0;
        }
        ev_changed.send(GameModeChanged {
            new_mode: *game_mode,
        });
        info!("GameMode: {:?}", *game_mode);
    }
}

pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let new_state = match *game_state {
            GameState::Playing => GameState::Paused,
            GameState::Paused => GameState::Playing,
        };
        *game_state = new_state;

        if let Ok(mut window) = windows.get_single_mut() {
            match new_state {
                GameState::Playing => {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
                GameState::Paused => {
                    window.cursor_options.grab_mode = CursorGrabMode::None;
                    window.cursor_options.visible = true;
                }
            }
        }
    }
}

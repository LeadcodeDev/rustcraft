use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;

use crate::ClientTransportRes;
use crate::avatar::CameraMode;
use crate::events::PlayerMovedEvent;
use crate::world::chunk::ChunkMap;

use rustcraft_protocol::physics::{
    GRAVITY, JUMP_VELOCITY, TERMINAL_VELOCITY, is_on_ground, move_with_collision,
};
use rustcraft_protocol::protocol::ClientMessage;

pub use rustcraft_protocol::physics::EYE_HEIGHT;

#[derive(Component)]
pub struct FlyCam;

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Component)]
pub struct Player {
    pub position: Vec3,
    pub velocity_y: f32,
    pub grounded: bool,
}

impl Player {
    pub fn location(&self, transform: &Transform) -> Location {
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        Location {
            x: self.position.x,
            y: self.position.y,
            z: self.position.z,
            yaw,
            pitch,
        }
    }
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
    InInventory,
}

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
    camera_mode: Res<CameraMode>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    settings: Res<CameraSettings>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    if *game_state != GameState::Playing {
        return;
    }
    if mouse_motion.delta == Vec2::ZERO {
        return;
    }

    // In first person, limit downward pitch to avoid clipping into body
    let max_down = match *camera_mode {
        CameraMode::FirstPerson => -1.3,
        CameraMode::ThirdPerson => -1.54,
    };

    for mut transform in &mut query {
        let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        yaw -= mouse_motion.delta.x * settings.sensitivity;
        pitch -= mouse_motion.delta.y * settings.sensitivity;
        pitch = pitch.clamp(max_down, 1.54);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

pub fn camera_movement(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<CameraSettings>,
    time: Res<Time>,
    chunk_map: Res<ChunkMap>,
    game_mode: Res<GameMode>,
    transport: Res<ClientTransportRes>,
    mut ev_moved: EventWriter<PlayerMovedEvent>,
    mut query: Query<(&mut Transform, &mut Player), With<FlyCam>>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    let dt = time.delta_secs();

    for (mut transform, mut player) in &mut query {
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

        let forward_pressed = keys.pressed(KeyCode::KeyW);
        let backward_pressed = keys.pressed(KeyCode::KeyS);
        let right_pressed = keys.pressed(KeyCode::KeyD);
        let left_pressed = keys.pressed(KeyCode::KeyA);
        let jump_pressed = keys.pressed(KeyCode::Space);
        let sneak_pressed = keys.pressed(KeyCode::ShiftLeft);

        let has_input = forward_pressed
            || backward_pressed
            || right_pressed
            || left_pressed
            || jump_pressed
            || sneak_pressed;

        // Only send input to server when there is actual input or player is airborne
        let needs_server_update = has_input || !player.grounded;
        if needs_server_update {
            transport.0.send(ClientMessage::InputCommand {
                sequence: 0,
                dt,
                yaw,
                pitch,
                forward: forward_pressed,
                backward: backward_pressed,
                left: left_pressed,
                right: right_pressed,
                jump: jump_pressed,
                sneak: sneak_pressed,
            });
        }

        // Client-side prediction: apply movement locally for instant feedback
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();

        let delta = match *game_mode {
            GameMode::Creative => {
                let mut velocity = Vec3::ZERO;
                if forward_pressed {
                    velocity += forward;
                }
                if backward_pressed {
                    velocity -= forward;
                }
                if right_pressed {
                    velocity += right;
                }
                if left_pressed {
                    velocity -= right;
                }
                if jump_pressed {
                    velocity += Vec3::Y;
                }
                if sneak_pressed {
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
                if forward_pressed {
                    horizontal += forward_xz;
                }
                if backward_pressed {
                    horizontal -= forward_xz;
                }
                if right_pressed {
                    horizontal += right_xz;
                }
                if left_pressed {
                    horizontal -= right_xz;
                }
                if horizontal != Vec3::ZERO {
                    horizontal = horizontal.normalize();
                }

                player.grounded = is_on_ground(player.position, &chunk_map.0);

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

        // Skip physics when at rest (grounded, no input) to avoid micro-jitter
        if !needs_server_update && player.grounded {
            transform.translation = player.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
            continue;
        }

        let old_pos = player.position;
        let (new_pos, hit_floor, hit_ceiling) =
            move_with_collision(player.position, delta, &chunk_map.0);
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
            ev_moved.send(PlayerMovedEvent {
                old_position: old_pos,
                new_position: player.position,
                player: player.location(&transform),
            });
        }
        transform.translation = player.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
    }
}

pub fn toggle_gamemode(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    transport: Res<ClientTransportRes>,
) {
    if *game_state != GameState::Playing {
        return;
    }
    if keys.just_pressed(KeyCode::F1) {
        transport.0.send(ClientMessage::ToggleGameMode);
    }
}

pub fn pause_on_focus_lost(
    mut game_state: ResMut<GameState>,
    mut focus_events: EventReader<bevy::window::WindowFocused>,
) {
    for event in focus_events.read() {
        if !event.focused && *game_state != GameState::Paused {
            *game_state = GameState::Paused;
        }
    }
}

/// Continuously enforce cursor state to match GameState.
/// Prevents macOS/Bevy from re-locking cursor when window regains focus.
pub fn enforce_cursor_state(game_state: Res<GameState>, mut windows: Query<&mut Window>) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    match *game_state {
        GameState::Playing => {
            if window.cursor_options.grab_mode != CursorGrabMode::Locked {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
        GameState::Paused | GameState::InInventory => {
            if window.cursor_options.grab_mode != CursorGrabMode::None {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            }
        }
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
            GameState::Paused | GameState::InInventory => GameState::Playing,
        };
        *game_state = new_state;

        if let Ok(mut window) = windows.get_single_mut() {
            match new_state {
                GameState::Playing => {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
                GameState::Paused | GameState::InInventory => {
                    window.cursor_options.grab_mode = CursorGrabMode::None;
                    window.cursor_options.visible = true;
                }
            }
        }
    }
}

pub fn toggle_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }

    let new_state = match *game_state {
        GameState::Playing => GameState::InInventory,
        GameState::InInventory => GameState::Playing,
        GameState::Paused => return,
    };
    *game_state = new_state;

    if let Ok(mut window) = windows.get_single_mut() {
        match new_state {
            GameState::Playing => {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
            GameState::InInventory => {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            }
            GameState::Paused => {}
        }
    }
}

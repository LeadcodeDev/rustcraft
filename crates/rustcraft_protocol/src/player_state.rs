use bevy_math::Vec3;

use crate::game_mode::GameMode;

pub struct PlayerState {
    pub position: Vec3,
    pub velocity_y: f32,
    pub grounded: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub game_mode: GameMode,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::new(64.0, 40.0 - 1.7, 64.0),
            velocity_y: 0.0,
            grounded: false,
            yaw: 0.0,
            pitch: 0.0,
            game_mode: GameMode::default(),
        }
    }
}

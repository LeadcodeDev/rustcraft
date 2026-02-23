pub mod camera;

use bevy::prelude::*;
use camera::{
    CameraSettings, GameMode, GameState, camera_look, camera_movement, initial_cursor_grab,
    spawn_camera, toggle_gamemode, toggle_pause,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .init_resource::<GameMode>()
            .init_resource::<GameState>()
            .add_systems(Startup, (spawn_camera, initial_cursor_grab))
            .add_systems(
                Update,
                (
                    camera_look,
                    camera_movement.after(camera_look),
                    toggle_pause,
                    toggle_gamemode,
                ),
            );
    }
}

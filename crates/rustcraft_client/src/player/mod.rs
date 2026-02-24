pub mod camera;

use bevy::prelude::*;
use camera::{
    CameraSettings, GameMode, GameState, camera_look, camera_movement, enforce_cursor_state,
    initial_cursor_grab, pause_on_focus_lost, spawn_camera, toggle_gamemode, toggle_inventory,
    toggle_pause,
};

use crate::app_state::AppState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .init_resource::<GameMode>()
            .init_resource::<GameState>()
            .add_systems(
                OnEnter(AppState::InGame),
                (spawn_camera, initial_cursor_grab),
            )
            .add_systems(
                Update,
                (
                    camera_look,
                    camera_movement.after(camera_look),
                    toggle_pause,
                    toggle_inventory,
                    toggle_gamemode,
                    pause_on_focus_lost,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Last,
                enforce_cursor_state.run_if(in_state(AppState::InGame)),
            );
    }
}

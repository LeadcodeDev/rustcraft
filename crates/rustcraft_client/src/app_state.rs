use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    InGame,
}

/// Configuration set by the main menu before transitioning to InGame.
#[derive(Resource, Clone)]
pub enum ConnectionConfig {
    Solo {
        world_name: String,
        seed: u32,
        player_name: String,
    },
    Multi {
        address: String,
        auth_code: String,
        player_name: String,
    },
}

/// Marker resource indicating the embedded server is running (solo mode).
#[derive(Resource)]
pub struct SoloMode;

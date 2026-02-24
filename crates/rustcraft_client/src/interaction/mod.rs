pub mod raycast;

use bevy::prelude::*;
use raycast::{
    DebugOverlayVisible, DropKeyState, block_interaction, drop_active_item, spawn_crosshair,
    spawn_debug_overlay, toggle_debug_overlay, update_debug_overlay,
};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlayVisible>()
            .init_resource::<DropKeyState>()
            .add_systems(Startup, (spawn_crosshair, spawn_debug_overlay))
            .add_systems(
                Update,
                (
                    block_interaction,
                    drop_active_item,
                    toggle_debug_overlay,
                    update_debug_overlay,
                ),
            );
    }
}

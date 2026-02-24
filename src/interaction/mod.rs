pub mod raycast;

use bevy::prelude::*;
use raycast::{
    DebugOverlayVisible, block_interaction, spawn_crosshair, spawn_debug_overlay,
    toggle_debug_overlay, update_debug_overlay,
};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlayVisible>()
            .add_systems(Startup, (spawn_crosshair, spawn_debug_overlay))
            .add_systems(
                Update,
                (
                    block_interaction,
                    toggle_debug_overlay,
                    update_debug_overlay,
                ),
            );
    }
}

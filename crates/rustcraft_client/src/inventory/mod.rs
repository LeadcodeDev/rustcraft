use std::ops::{Deref, DerefMut};

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::player::camera::GameState;

pub use rustcraft_protocol::inventory::{ItemStack, MAX_STACK};

use rustcraft_protocol::inventory::Inventory as ProtocolInventory;

/// Bevy Resource wrapper around the protocol Inventory.
#[derive(Resource)]
pub struct Inventory(pub ProtocolInventory);

impl Default for Inventory {
    fn default() -> Self {
        Self(ProtocolInventory::default())
    }
}

impl Deref for Inventory {
    type Target = ProtocolInventory;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Inventory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .add_systems(
                Update,
                scroll_hotbar.run_if(in_state(crate::app_state::AppState::InGame)),
            );
    }
}

fn scroll_hotbar(
    game_state: Res<GameState>,
    mut mouse_wheel: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    for event in mouse_wheel.read() {
        // macOS natural scrolling: delta.y > 0 = next slot, delta.y < 0 = previous slot
        if event.y > 0.0 {
            inventory.active_slot = (inventory.active_slot + 1) % 9;
        } else if event.y < 0.0 {
            if inventory.active_slot == 0 {
                inventory.active_slot = 8;
            } else {
                inventory.active_slot -= 1;
            }
        }
    }

    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if keys.just_pressed(key) {
            inventory.active_slot = slot;
        }
    }
}

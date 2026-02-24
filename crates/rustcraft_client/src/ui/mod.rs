pub mod block_preview;
pub mod hotbar;
pub mod inventory_screen;
pub mod main_menu;
pub mod pause_menu;
pub mod text_input;

use bevy::prelude::*;
use block_preview::setup_block_previews;
use hotbar::{show_hide_hotbar, spawn_hotbar, update_hotbar};
use inventory_screen::{
    DragState, drag_and_drop, show_hide_inventory_screen, spawn_inventory_screen,
    update_inventory_screen,
};
use pause_menu::{
    button_hover, handle_quit_button, handle_quit_to_menu_button, handle_resume_button,
    show_hide_pause_menu, spawn_pause_menu,
};

use crate::app_state::AppState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DragState>()
            .add_systems(
                OnEnter(AppState::InGame),
                (
                    setup_block_previews,
                    spawn_pause_menu,
                    spawn_hotbar,
                    spawn_inventory_screen,
                ),
            )
            .add_systems(
                Update,
                (
                    show_hide_pause_menu,
                    handle_resume_button,
                    handle_quit_button,
                    handle_quit_to_menu_button,
                    button_hover,
                    show_hide_hotbar,
                    update_hotbar,
                    show_hide_inventory_screen,
                    update_inventory_screen,
                    drag_and_drop,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

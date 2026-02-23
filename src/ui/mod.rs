pub mod pause_menu;

use bevy::prelude::*;
use pause_menu::{
    button_hover, handle_quit_button, handle_resume_button, show_hide_pause_menu, spawn_pause_menu,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_pause_menu).add_systems(
            Update,
            (
                show_hide_pause_menu,
                handle_resume_button,
                handle_quit_button,
                button_hover,
            ),
        );
    }
}

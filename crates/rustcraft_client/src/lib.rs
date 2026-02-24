pub mod app_state;
pub mod avatar;
pub mod dropped_item;
pub mod events;
pub mod interaction;
pub mod inventory;
pub mod network;
pub mod player;
pub mod render;
pub mod session;
pub mod ui;
pub mod world;

use std::sync::Mutex;

use bevy::prelude::*;
use rustcraft_protocol::transport::ClientTransport;

use app_state::AppState;
use events::EventsPlugin;
use session::{cleanup_game_session, client_connect, start_game_session};
use ui::main_menu::{MainMenuPlugin, MenuScreen};
use ui::text_input::TextInputPlugin;

/// Bevy Resource wrapping a boxed ClientTransport.
#[derive(Resource)]
pub struct ClientTransportRes(pub Box<dyn ClientTransport>);

/// The local player's ID assigned by the server.
#[derive(Resource, Default)]
pub struct LocalPlayerId(pub Option<u64>);

/// Authentication parameters for connecting to the server.
#[derive(Resource, Clone)]
pub struct AuthConfig {
    pub auth_code: String,
    pub player_name: String,
}

/// The client plugin composes all client-side functionality:
/// rendering, input, UI, prediction, etc.
pub struct ClientPlugin {
    event_plugins: Mutex<Vec<Box<dyn events::RustcraftPlugin>>>,
}

impl ClientPlugin {
    pub fn new() -> Self {
        Self {
            event_plugins: Mutex::new(Vec::new()),
        }
    }

    pub fn with_plugin(self, plugin: impl events::RustcraftPlugin) -> Self {
        self.event_plugins.lock().unwrap().push(Box::new(plugin));
        self
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let event_plugins = self.event_plugins.lock().unwrap().drain(..).collect();

        app.init_state::<AppState>()
            .init_state::<MenuScreen>()
            .enable_state_scoped_entities::<AppState>()
            // Sub-plugins (all internally gated on AppState::InGame)
            .add_plugins(EventsPlugin::new_with(event_plugins))
            .add_plugins(world::WorldPlugin)
            .add_plugins(render::RenderPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(inventory::InventoryPlugin)
            .add_plugins(interaction::InteractionPlugin)
            .add_plugins(ui::UiPlugin)
            .add_plugins(dropped_item::DroppedItemPlugin)
            .add_plugins(avatar::AvatarPlugin)
            // Menu
            .add_plugins(MainMenuPlugin)
            .add_plugins(TextInputPlugin)
            // Session lifecycle
            .add_systems(OnEnter(AppState::InGame), start_game_session)
            .add_systems(OnExit(AppState::InGame), cleanup_game_session)
            .add_systems(
                Update,
                (
                    client_connect.run_if(in_state(AppState::InGame)),
                    network::client_receive_messages
                        .run_if(in_state(AppState::InGame)),
                ),
            );

        // Register embedded server systems (gated on resource_exists)
        rustcraft_server::ServerPlugin::register_systems(app);
    }
}

pub mod avatar;
pub mod dropped_item;
pub mod events;
pub mod interaction;
pub mod inventory;
pub mod network;
pub mod player;
pub mod render;
pub mod ui;
pub mod world;

use std::sync::Mutex;

use bevy::prelude::*;
use rustcraft_protocol::transport::ClientTransport;

use events::EventsPlugin;

/// Bevy Resource wrapping a boxed ClientTransport.
#[derive(Resource)]
pub struct ClientTransportRes(pub Box<dyn ClientTransport>);

/// The client plugin composes all client-side functionality:
/// rendering, input, UI, prediction, etc.
pub struct ClientPlugin {
    transport: Mutex<Option<Box<dyn ClientTransport>>>,
    event_plugins: Mutex<Vec<Box<dyn events::RustcraftPlugin>>>,
}

impl ClientPlugin {
    pub fn new(transport: Box<dyn ClientTransport>) -> Self {
        Self {
            transport: Mutex::new(Some(transport)),
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
        let transport = self
            .transport
            .lock()
            .unwrap()
            .take()
            .expect("ClientPlugin transport already taken");

        let event_plugins = self.event_plugins.lock().unwrap().drain(..).collect();

        app.insert_resource(ClientTransportRes(transport))
            .add_plugins(EventsPlugin::new_with(event_plugins))
            .add_plugins(world::WorldPlugin)
            .add_plugins(render::RenderPlugin)
            .add_plugins(player::PlayerPlugin)
            .add_plugins(inventory::InventoryPlugin)
            .add_plugins(interaction::InteractionPlugin)
            .add_plugins(ui::UiPlugin)
            .add_plugins(dropped_item::DroppedItemPlugin)
            .add_plugins(avatar::AvatarPlugin)
            .add_systems(Update, network::client_receive_messages);
    }
}

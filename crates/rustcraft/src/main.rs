use bevy::prelude::*;
use rustcraft_macros::craft_plugin;

use rustcraft_client::events;
use rustcraft_protocol::transport::create_local_transport;

struct LogPlugin;

#[craft_plugin]
impl LogPlugin {
    #[Event::PlayerMoved]
    fn on_move(&self, event: &events::PlayerMovedEvent) {
        info!(
            "Player moved to ({:.1}, {:.1}, {:.1})",
            event.player.x, event.player.y, event.player.z
        );
    }

    #[Event::BlockPlaced]
    fn on_block_placed(&self, event: &events::BlockPlacedEvent) {
        info!(
            "Player at ({:.1}, {:.1}, {:.1}) placed {:?} at ({}, {}, {})",
            event.player.x,
            event.player.y,
            event.player.z,
            event.block_type,
            event.position.x,
            event.position.y,
            event.position.z
        );
    }

    #[Event::BlockRemoved]
    fn on_block_removed(&self, event: &events::BlockRemovedEvent) {
        info!(
            "Player at ({:.1}, {:.1}, {:.1}) broke {:?} at ({}, {}, {})",
            event.player.x,
            event.player.y,
            event.player.z,
            event.block_type,
            event.position.x,
            event.position.y,
            event.position.z
        );
    }

    #[Event::ItemDroppedToWorld]
    fn on_item_dropped(&self, event: &events::ItemDroppedToWorldEvent) {
        info!(
            "Player at ({:.1}, {:.1}, {:.1}) dropped {:?} x{} at ({:.1}, {:.1}, {:.1})",
            event.player.x,
            event.player.y,
            event.player.z,
            event.block_type,
            event.count,
            event.position.x,
            event.position.y,
            event.position.z
        );
    }
}

fn main() {
    let (client_transport, server_transport) = create_local_transport();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rustcraft".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(rustcraft_server::ServerPlugin::new(
            server_transport,
            "Default World",
            42,
        ))
        .add_plugins(
            rustcraft_client::ClientPlugin::new(Box::new(client_transport))
                .with_plugin(LogPlugin),
        )
        .add_systems(Startup, setup_lighting)
        .run();
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.3, 0.0)),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
    });
}

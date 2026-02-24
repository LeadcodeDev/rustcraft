use bevy::prelude::*;
use clap::Parser;
use rustcraft_macros::craft_plugin;

use rustcraft_client::events;
use rustcraft_protocol::tcp_transport::TcpClientTransport;
use rustcraft_protocol::transport::create_local_transport;

#[derive(Parser)]
#[command(name = "rustcraft")]
#[command(about = "Rustcraft — Minecraft-like voxel game")]
struct Args {
    /// Connect to a remote server (host:port)
    #[arg(long)]
    connect: Option<String>,

    /// Authentication code (required when connecting to a remote server)
    #[arg(long)]
    code: Option<String>,

    /// Player name
    #[arg(long, default_value = "Player")]
    name: String,

    /// World seed (solo mode only)
    #[arg(long, default_value_t = 42)]
    seed: u32,

    /// World name (solo mode only)
    #[arg(long, default_value = "Default World")]
    world: String,
}

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
    let args = Args::parse();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Rustcraft".into(),
            ..default()
        }),
        ..default()
    }));

    match args.connect {
        // Network mode: connect to a remote server
        Some(addr) => {
            let code = args
                .code
                .expect("--code is required when using --connect");

            let transport = TcpClientTransport::connect(&addr)
                .unwrap_or_else(|e| panic!("Failed to connect to {}: {}", addr, e));

            app.add_plugins(
                rustcraft_client::ClientPlugin::new(Box::new(transport), code, args.name)
                    .with_plugin(LogPlugin),
            );
        }
        // Solo mode: embedded server + local transport
        None => {
            let (client_transport, server_transport) = create_local_transport();

            let server_plugin =
                rustcraft_server::ServerPlugin::new(server_transport, args.world, args.seed);
            let auth_code = server_plugin.auth_code().to_string();

            app.add_plugins(server_plugin);
            app.add_plugins(
                rustcraft_client::ClientPlugin::new(
                    Box::new(client_transport),
                    auth_code,
                    args.name,
                )
                .with_plugin(LogPlugin),
            );
        }
    }

    app.add_systems(Startup, setup_lighting);
    app.run();
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

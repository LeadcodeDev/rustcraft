use std::path::PathBuf;

use bevy::prelude::*;
use clap::Parser;

use rustcraft_protocol::tcp_transport::TcpServerTransport;
use rustcraft_server::world_session::WorldSession;
use rustcraft_server::ServerPlugin;

#[derive(Parser)]
#[command(name = "rustcraft_dedicated_server")]
#[command(about = "Rustcraft dedicated server (headless)")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 25565)]
    port: u16,

    /// World seed
    #[arg(short, long, default_value_t = 42)]
    seed: u32,

    /// World name
    #[arg(short, long, default_value = "World")]
    world: String,

    /// Path to store world data
    #[arg(long, default_value = "worlds")]
    save_path: String,
}

fn main() {
    let args = Args::parse();

    let world_path = PathBuf::from(&args.save_path).join(&args.world);
    let session = WorldSession::load_or_create(world_path, args.world.clone(), args.seed);

    println!("Auth code: {}", session.auth_code);
    println!(
        "World '{}' (seed={})",
        session.name, session.seed
    );

    let addr = format!("0.0.0.0:{}", args.port);
    let transport = TcpServerTransport::new(&addr);
    println!("Listening on {}", addr);

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(bevy::log::LogPlugin::default())
        .add_plugins(ServerPlugin::with_session(transport, session))
        .run();
}

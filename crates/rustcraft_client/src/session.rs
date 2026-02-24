use std::path::PathBuf;

use bevy::prelude::*;

use rustcraft_protocol::protocol::ClientMessage;
use rustcraft_protocol::transport::create_local_transport;
use rustcraft_protocol::tcp_transport::TcpClientTransport;
use rustcraft_server::systems::ServerTransportRes;
use rustcraft_server::world_session::WorldSession;

use crate::app_state::{ConnectionConfig, SoloMode};
use crate::interaction::raycast::DebugOverlayVisible;
use crate::network::RemotePlayerStates;
use crate::render::SpawnedChunks;
use crate::inventory::Inventory;
use crate::player::camera::{GameMode, GameState};
use crate::ui::block_preview::BlockPreviews;
use crate::ui::inventory_screen::DragState;
use crate::world::chunk::ChunkMap;
use crate::{AuthConfig, ClientTransportRes, LocalPlayerId};

/// Exclusive system that runs on OnEnter(AppState::InGame).
/// Creates transport and inserts all game resources synchronously.
pub fn start_game_session(world: &mut World) {
    let config = world.resource::<ConnectionConfig>().clone();

    match config {
        ConnectionConfig::Solo {
            world_name,
            seed,
            player_name,
        } => {
            let (client_transport, server_transport) = create_local_transport();
            let world_path = PathBuf::from("worlds").join(&world_name);
            let session = WorldSession::load_or_create(world_path, world_name, seed);
            let auth_code = session.auth_code.clone();

            world.insert_resource(ServerTransportRes(Box::new(server_transport)));
            world.insert_resource(session);
            world.insert_resource(ClientTransportRes(Box::new(client_transport)));
            world.insert_resource(AuthConfig {
                auth_code,
                player_name,
            });
            world.insert_resource(SoloMode);
        }
        ConnectionConfig::Multi {
            address,
            auth_code,
            player_name,
        } => {
            let transport = TcpClientTransport::connect(&address)
                .unwrap_or_else(|e| panic!("Failed to connect to {}: {}", address, e));

            world.insert_resource(ClientTransportRes(Box::new(transport)));
            world.insert_resource(AuthConfig {
                auth_code,
                player_name,
            });
        }
    }

    world.insert_resource(LocalPlayerId::default());
    world.insert_resource(RemotePlayerStates::default());
}

/// Marker resource: inserted after Connect is sent to prevent re-sending.
#[derive(Resource)]
pub struct HasConnected;

/// Sends the Connect message once when entering InGame.
pub fn client_connect(
    mut commands: Commands,
    transport: Res<ClientTransportRes>,
    auth: Res<AuthConfig>,
    has_connected: Option<Res<HasConnected>>,
) {
    if has_connected.is_some() {
        return;
    }
    commands.insert_resource(HasConnected);

    transport.0.send(ClientMessage::Connect {
        auth_code: auth.auth_code.clone(),
        player_name: auth.player_name.clone(),
    });
}

/// Cleanup system that runs on OnExit(AppState::InGame).
/// Saves world, disconnects, and removes all game resources.
pub fn cleanup_game_session(world: &mut World) {
    // Save world if solo mode
    if world.remove_resource::<SoloMode>().is_some() {
        if let Some(mut session) = world.get_resource_mut::<WorldSession>() {
            session.save_to_disk();
        }
    }

    // Send disconnect
    if let Some(transport) = world.get_resource::<ClientTransportRes>() {
        transport.0.send(ClientMessage::Disconnect);
    }

    // Remove game resources
    world.remove_resource::<ClientTransportRes>();
    world.remove_resource::<ServerTransportRes>();
    world.remove_resource::<WorldSession>();
    world.remove_resource::<LocalPlayerId>();
    world.remove_resource::<RemotePlayerStates>();
    world.remove_resource::<AuthConfig>();
    world.remove_resource::<ConnectionConfig>();
    world.remove_resource::<HasConnected>();
    world.remove_resource::<BlockPreviews>();

    // Reset mutable state resources
    if let Some(mut chunk_map) = world.get_resource_mut::<ChunkMap>() {
        *chunk_map = ChunkMap::default();
    }
    if let Some(mut inventory) = world.get_resource_mut::<Inventory>() {
        *inventory = Inventory::default();
    }
    if let Some(mut spawned) = world.get_resource_mut::<SpawnedChunks>() {
        spawned.0.clear();
    }
    if let Some(mut game_state) = world.get_resource_mut::<GameState>() {
        *game_state = GameState::default();
    }
    if let Some(mut game_mode) = world.get_resource_mut::<GameMode>() {
        *game_mode = GameMode::default();
    }
    if let Some(mut drag_state) = world.get_resource_mut::<DragState>() {
        drag_state.clear();
    }
    if let Some(mut debug_visible) = world.get_resource_mut::<DebugOverlayVisible>() {
        *debug_visible = DebugOverlayVisible::default();
    }
}

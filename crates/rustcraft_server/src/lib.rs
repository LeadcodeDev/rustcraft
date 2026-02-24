pub mod systems;
pub mod world_session;

use std::path::PathBuf;
use std::sync::Mutex;

use bevy::prelude::*;

use rustcraft_protocol::transport::ServerTransport;

use systems::{
    ServerTransportRes, server_auto_save, server_dropped_item_physics, server_pickup_items,
    server_process_messages, server_stream_chunks,
};
use world_session::WorldSession;

pub struct ServerPlugin {
    transport: Mutex<Option<Box<dyn ServerTransport>>>,
    session: Mutex<Option<WorldSession>>,
    auth_code: String,
}

impl ServerPlugin {
    pub fn new(
        transport: impl ServerTransport,
        world_name: impl Into<String>,
        seed: u32,
    ) -> Self {
        let name = world_name.into();
        let world_path = PathBuf::from("worlds").join(&name);
        let session = WorldSession::load_or_create(world_path, name, seed);
        let auth_code = session.auth_code.clone();
        Self {
            transport: Mutex::new(Some(Box::new(transport))),
            session: Mutex::new(Some(session)),
            auth_code,
        }
    }

    /// Create a ServerPlugin with a pre-built session (for dedicated server).
    pub fn with_session(transport: impl ServerTransport, session: WorldSession) -> Self {
        let auth_code = session.auth_code.clone();
        Self {
            transport: Mutex::new(Some(Box::new(transport))),
            session: Mutex::new(Some(session)),
            auth_code,
        }
    }

    /// Get the auth code for this server.
    pub fn auth_code(&self) -> &str {
        &self.auth_code
    }

    /// Register server systems gated on resource existence.
    /// Resources are inserted later when a solo game session starts.
    pub fn register_systems(app: &mut App) {
        app.add_systems(
            Update,
            (
                server_process_messages,
                server_stream_chunks.after(server_process_messages),
                server_dropped_item_physics.after(server_stream_chunks),
                server_pickup_items.after(server_dropped_item_physics),
                server_auto_save.after(server_pickup_items),
            )
                .run_if(resource_exists::<ServerTransportRes>)
                .run_if(resource_exists::<WorldSession>),
        );
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let transport = self
            .transport
            .lock()
            .unwrap()
            .take()
            .expect("ServerPlugin transport already taken");

        let session = self
            .session
            .lock()
            .unwrap()
            .take()
            .expect("ServerPlugin session already taken");

        info!("Auth code: {}", session.auth_code);
        info!("World '{}' (seed={})", session.name, session.seed);

        app.insert_resource(ServerTransportRes(transport))
            .insert_resource(session)
            .add_systems(
                Update,
                (
                    server_process_messages,
                    server_stream_chunks.after(server_process_messages),
                    server_dropped_item_physics.after(server_stream_chunks),
                    server_pickup_items.after(server_dropped_item_physics),
                    server_auto_save.after(server_pickup_items),
                ),
            );
    }
}

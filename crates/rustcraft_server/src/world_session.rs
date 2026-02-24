use std::collections::HashMap;

use bevy::prelude::*;

use rustcraft_protocol::chunk::ChunkMap;
use rustcraft_protocol::inventory::Inventory;
use rustcraft_protocol::player_state::PlayerState;

/// Dropped item state tracked by the server.
pub struct DroppedItemState {
    pub stack: rustcraft_protocol::inventory::ItemStack,
    pub position: bevy::math::Vec3,
    pub velocity: bevy::math::Vec3,
    pub grounded: bool,
    pub age: f32,
}

/// Server-side world session containing all authoritative game state.
#[derive(Resource)]
pub struct WorldSession {
    pub name: String,
    pub seed: u32,
    pub tick: u64,
    pub chunk_map: ChunkMap,
    pub players: HashMap<u64, PlayerState>,
    pub inventories: HashMap<u64, Inventory>,
    pub dropped_items: HashMap<u64, DroppedItemState>,
    pub next_entity_id: u64,
}

impl WorldSession {
    pub fn new(name: String, seed: u32) -> Self {
        let mut chunk_map = ChunkMap::default();
        rustcraft_protocol::generation::generate_world(&mut chunk_map, seed);

        let mut players = HashMap::new();
        players.insert(0, PlayerState::default());

        let mut inventories = HashMap::new();
        inventories.insert(0, Inventory::default());

        Self {
            name,
            seed,
            tick: 0,
            chunk_map,
            players,
            inventories,
            dropped_items: HashMap::new(),
            next_entity_id: 1,
        }
    }
}

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use noise::Perlin;

use rustcraft_protocol::block::BlockType;
use rustcraft_protocol::chunk::{BLOCKS_PER_CHUNK, CHUNK_SIZE, Chunk, ChunkMap, ChunkPos};
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

/// Metadata saved to disk for a world.
#[derive(serde::Serialize, serde::Deserialize)]
struct WorldMeta {
    seed: u32,
    tick: u64,
    next_entity_id: u64,
}

/// Server-side world session containing all authoritative game state.
#[derive(Resource)]
pub struct WorldSession {
    pub name: String,
    pub seed: u32,
    pub tick: u64,
    pub auth_code: String,
    pub world_path: PathBuf,
    pub perlin: Perlin,
    pub chunk_map: ChunkMap,
    pub players: HashMap<u64, PlayerState>,
    pub player_names: HashMap<u64, String>,
    pub inventories: HashMap<u64, Inventory>,
    pub dropped_items: HashMap<u64, DroppedItemState>,
    pub next_entity_id: u64,
    /// Tracks which chunks each player has received.
    pub loaded_chunks_per_player: HashMap<u64, HashSet<ChunkPos>>,
    /// Ticks since last auto-save.
    pub ticks_since_save: u64,
}

impl WorldSession {
    /// Generate a random 6-character alphanumeric auth code.
    fn generate_auth_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'A' + idx - 10) as char
                }
            })
            .collect()
    }

    /// Create a new world session (no players initially).
    pub fn new(name: String, seed: u32, world_path: PathBuf) -> Self {
        let perlin = Perlin::new(seed);

        Self {
            name,
            seed,
            tick: 0,
            auth_code: Self::generate_auth_code(),
            world_path,
            perlin,
            chunk_map: ChunkMap::default(),
            players: HashMap::new(),
            player_names: HashMap::new(),
            inventories: HashMap::new(),
            dropped_items: HashMap::new(),
            next_entity_id: 1,
            loaded_chunks_per_player: HashMap::new(),
            ticks_since_save: 0,
        }
    }

    /// Load a world from disk or create a new one if it doesn't exist.
    pub fn load_or_create(world_path: PathBuf, name: String, seed: u32) -> Self {
        let meta_path = world_path.join("world.dat");
        if meta_path.exists() {
            if let Some(session) = Self::load_from_disk(&world_path, name.clone()) {
                return session;
            }
        }
        Self::new(name, seed, world_path)
    }

    /// Load world metadata from disk. Chunks are loaded on-demand.
    fn load_from_disk(world_path: &Path, name: String) -> Option<Self> {
        let meta_path = world_path.join("world.dat");
        let data = fs::read(&meta_path).ok()?;
        let meta: WorldMeta = bincode::deserialize(&data).ok()?;

        let perlin = Perlin::new(meta.seed);

        Some(Self {
            name,
            seed: meta.seed,
            tick: meta.tick,
            auth_code: Self::generate_auth_code(),
            world_path: world_path.to_path_buf(),
            perlin,
            chunk_map: ChunkMap::default(),
            players: HashMap::new(),
            player_names: HashMap::new(),
            inventories: HashMap::new(),
            dropped_items: HashMap::new(),
            next_entity_id: meta.next_entity_id,
            loaded_chunks_per_player: HashMap::new(),
            ticks_since_save: 0,
        })
    }

    /// Save world metadata and all dirty chunks to disk.
    pub fn save_to_disk(&mut self) {
        let _ = fs::create_dir_all(&self.world_path);
        let chunks_dir = self.world_path.join("chunks");
        let _ = fs::create_dir_all(&chunks_dir);

        // Save metadata
        let meta = WorldMeta {
            seed: self.seed,
            tick: self.tick,
            next_entity_id: self.next_entity_id,
        };
        if let Ok(data) = bincode::serialize(&meta) {
            let _ = fs::write(self.world_path.join("world.dat"), data);
        }

        // Save dirty chunks
        for (&pos, chunk) in &mut self.chunk_map.chunks {
            if chunk.dirty {
                let path = chunks_dir.join(format!("{}_{}.dat", pos.0, pos.1));
                if let Ok(data) = bincode::serialize(&chunk.blocks) {
                    let _ = fs::write(path, data);
                }
                chunk.dirty = false;
            }
        }
    }

    /// Ensure a chunk is loaded in memory. If not present, try loading from disk
    /// or generate it with Perlin noise.
    pub fn ensure_chunk_loaded(&mut self, pos: ChunkPos) {
        if self.chunk_map.chunks.contains_key(&pos) {
            return;
        }

        // Try loading from disk
        let chunk_path = self
            .world_path
            .join("chunks")
            .join(format!("{}_{}.dat", pos.0, pos.1));
        if chunk_path.exists() {
            if let Ok(data) = fs::read(&chunk_path) {
                if let Ok(blocks) = bincode::deserialize::<Vec<BlockType>>(&data) {
                    if blocks.len() == BLOCKS_PER_CHUNK {
                        let mut chunk = Chunk::new();
                        chunk.blocks = blocks;
                        chunk.dirty = false;
                        self.chunk_map.chunks.insert(pos, chunk);
                        return;
                    }
                }
            }
        }

        // Generate with Perlin noise
        self.generate_chunk(pos);
    }

    /// Generate a single chunk using Perlin noise.
    fn generate_chunk(&mut self, pos: ChunkPos) {
        use noise::NoiseFn;

        const BASE_HEIGHT: f64 = 20.0;
        const AMPLITUDE: f64 = 15.0;
        const NOISE_SCALE: f64 = 0.02;
        const SAND_LEVEL: i32 = 14;

        let mut chunk = Chunk::new();

        for lx in 0..CHUNK_SIZE {
            for lz in 0..CHUNK_SIZE {
                let wx = pos.0 as f64 * CHUNK_SIZE as f64 + lx as f64;
                let wz = pos.1 as f64 * CHUNK_SIZE as f64 + lz as f64;

                let noise_val = self.perlin.get([wx * NOISE_SCALE, wz * NOISE_SCALE]);
                let height = (BASE_HEIGHT + noise_val * AMPLITUDE) as i32;
                let height =
                    height.clamp(1, rustcraft_protocol::chunk::CHUNK_HEIGHT as i32 - 1);

                for y in 0..=height {
                    let block = if y == height {
                        if height <= SAND_LEVEL {
                            BlockType::Sand
                        } else {
                            BlockType::Grass
                        }
                    } else if y >= height - 3 {
                        BlockType::Dirt
                    } else {
                        BlockType::Stone
                    };

                    chunk.set_block(lx, y as usize, lz, block);
                }
            }
        }

        chunk.dirty = false;
        self.chunk_map.chunks.insert(pos, chunk);
    }

    /// Add a player to the world. Returns a reference to the new PlayerState.
    pub fn add_player(&mut self, id: u64, name: String) -> &PlayerState {
        self.players.insert(id, PlayerState::default());
        self.player_names.insert(id, name);
        self.inventories.insert(id, Inventory::default());
        self.loaded_chunks_per_player.insert(id, HashSet::new());
        self.players.get(&id).unwrap()
    }

    /// Remove a player from the world.
    pub fn remove_player(&mut self, id: u64) {
        self.players.remove(&id);
        self.player_names.remove(&id);
        self.inventories.remove(&id);
        self.loaded_chunks_per_player.remove(&id);
    }

    /// Check if a chunk can be unloaded (no player has it in their set).
    pub fn can_unload_chunk(&self, pos: &ChunkPos) -> bool {
        for loaded in self.loaded_chunks_per_player.values() {
            if loaded.contains(pos) {
                return false;
            }
        }
        true
    }
}

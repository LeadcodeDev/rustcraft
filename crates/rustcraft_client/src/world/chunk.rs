use std::ops::{Deref, DerefMut};

use bevy::prelude::*;

pub use rustcraft_protocol::chunk::{CHUNK_HEIGHT, CHUNK_SIZE};
pub use rustcraft_protocol::chunk::ChunkPos;

use rustcraft_protocol::chunk::ChunkMap as ProtocolChunkMap;

/// Bevy Resource wrapper around the protocol ChunkMap.
#[derive(Resource, Default)]
pub struct ChunkMap(pub ProtocolChunkMap);

impl Deref for ChunkMap {
    type Target = ProtocolChunkMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChunkMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

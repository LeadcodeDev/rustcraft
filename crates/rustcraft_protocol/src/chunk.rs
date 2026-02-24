use std::collections::HashMap;

use bevy_math::Vec3;

use crate::block::BlockType;

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 64;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT;
pub const VIEW_DISTANCE: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos(pub i32, pub i32);

pub struct Chunk {
    pub blocks: Vec<BlockType>,
    pub dirty: bool,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![BlockType::Air; BLOCKS_PER_CHUNK],
            dirty: false,
        }
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        if x >= CHUNK_SIZE || y >= CHUNK_HEIGHT || z >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.blocks[Self::index(x, y, z)]
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockType) {
        if x >= CHUNK_SIZE || y >= CHUNK_HEIGHT || z >= CHUNK_SIZE {
            return;
        }
        self.blocks[Self::index(x, y, z)] = block;
        self.dirty = true;
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct ChunkMap {
    pub chunks: HashMap<ChunkPos, Chunk>,
}

impl ChunkMap {
    pub fn get_block(&self, wx: i32, wy: i32, wz: i32) -> BlockType {
        if wy < 0 || wy >= CHUNK_HEIGHT as i32 {
            return BlockType::Air;
        }
        let cx = wx.div_euclid(CHUNK_SIZE as i32);
        let cz = wz.div_euclid(CHUNK_SIZE as i32);
        let lx = wx.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE as i32) as usize;
        self.chunks
            .get(&ChunkPos(cx, cz))
            .map(|c| c.get_block(lx, wy as usize, lz))
            .unwrap_or(BlockType::Air)
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, block: BlockType) {
        if wy < 0 || wy >= CHUNK_HEIGHT as i32 {
            return;
        }
        let cx = wx.div_euclid(CHUNK_SIZE as i32);
        let cz = wz.div_euclid(CHUNK_SIZE as i32);
        let lx = wx.rem_euclid(CHUNK_SIZE as i32) as usize;
        let lz = wz.rem_euclid(CHUNK_SIZE as i32) as usize;
        if let Some(chunk) = self.chunks.get_mut(&ChunkPos(cx, cz)) {
            chunk.set_block(lx, wy as usize, lz, block);
        }

        // Mark neighbor chunks dirty if block is on a border
        if lx == 0 {
            if let Some(c) = self.chunks.get_mut(&ChunkPos(cx - 1, cz)) {
                c.dirty = true;
            }
        }
        if lx == CHUNK_SIZE - 1 {
            if let Some(c) = self.chunks.get_mut(&ChunkPos(cx + 1, cz)) {
                c.dirty = true;
            }
        }
        if lz == 0 {
            if let Some(c) = self.chunks.get_mut(&ChunkPos(cx, cz - 1)) {
                c.dirty = true;
            }
        }
        if lz == CHUNK_SIZE - 1 {
            if let Some(c) = self.chunks.get_mut(&ChunkPos(cx, cz + 1)) {
                c.dirty = true;
            }
        }
    }
}

/// Returns all chunk positions within a square radius around a world position.
pub fn chunks_in_view_radius(pos: Vec3, radius: i32) -> Vec<ChunkPos> {
    let cx = (pos.x as i32).div_euclid(CHUNK_SIZE as i32);
    let cz = (pos.z as i32).div_euclid(CHUNK_SIZE as i32);
    let mut result = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);
    for x in (cx - radius)..=(cx + radius) {
        for z in (cz - radius)..=(cz + radius) {
            result.push(ChunkPos(x, z));
        }
    }
    result
}

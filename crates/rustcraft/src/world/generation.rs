use noise::{NoiseFn, Perlin};

use super::block::BlockType;
use super::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, Chunk, ChunkMap, ChunkPos};

const WORLD_CHUNKS: i32 = 8;
const BASE_HEIGHT: f64 = 20.0;
const AMPLITUDE: f64 = 15.0;
const NOISE_SCALE: f64 = 0.02;
const SAND_LEVEL: i32 = 14;
const SEED: u32 = 42;

pub fn generate_world(mut chunk_map: bevy::prelude::ResMut<ChunkMap>) {
    let perlin = Perlin::new(SEED);

    for cx in 0..WORLD_CHUNKS {
        for cz in 0..WORLD_CHUNKS {
            let mut chunk = Chunk::new();

            for lx in 0..CHUNK_SIZE {
                for lz in 0..CHUNK_SIZE {
                    let wx = cx as f64 * CHUNK_SIZE as f64 + lx as f64;
                    let wz = cz as f64 * CHUNK_SIZE as f64 + lz as f64;

                    let noise_val = perlin.get([wx * NOISE_SCALE, wz * NOISE_SCALE]);
                    let height = (BASE_HEIGHT + noise_val * AMPLITUDE) as i32;
                    let height = height.clamp(1, CHUNK_HEIGHT as i32 - 1);

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
            chunk_map.chunks.insert(ChunkPos(cx, cz), chunk);
        }
    }
}

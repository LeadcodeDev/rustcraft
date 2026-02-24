use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use rustcraft_protocol::block::BlockType;

use crate::world::block::BlockColor;
use crate::world::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, ChunkMap, ChunkPos};

/// Compute AO level (0-3) for a vertex given its 3 neighbor occupancies.
fn vertex_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        3
    } else {
        (side1 as u8) + (side2 as u8) + (corner as u8)
    }
}

const AO_CURVE: [f32; 4] = [1.0, 0.75, 0.55, 0.35];

// --- Chunk snapshot for off-thread meshing ---

const PAD: usize = 1;
const SNAP_X: usize = CHUNK_SIZE + 2 * PAD;
const SNAP_Z: usize = CHUNK_SIZE + 2 * PAD;
const SNAP_Y: usize = CHUNK_HEIGHT;

/// Self-contained block data for one chunk + 1-block border.
pub struct ChunkSnapshot {
    blocks: Vec<BlockType>,
}

impl ChunkSnapshot {
    fn index(x: i32, y: i32, z: i32) -> usize {
        let sx = (x + PAD as i32) as usize;
        let sy = y as usize;
        let sz = (z + PAD as i32) as usize;
        sx + sz * SNAP_X + sy * SNAP_X * SNAP_Z
    }

    pub fn from_chunk_map(chunk_pos: ChunkPos, chunk_map: &ChunkMap) -> Self {
        let total = SNAP_X * SNAP_Y * SNAP_Z;
        let mut blocks = vec![BlockType::Air; total];
        let base_x = chunk_pos.0 * CHUNK_SIZE as i32;
        let base_z = chunk_pos.1 * CHUNK_SIZE as i32;

        for y in 0..SNAP_Y as i32 {
            for z in -(PAD as i32)..(CHUNK_SIZE as i32 + PAD as i32) {
                for x in -(PAD as i32)..(CHUNK_SIZE as i32 + PAD as i32) {
                    blocks[Self::index(x, y, z)] = chunk_map.get_block(base_x + x, y, base_z + z);
                }
            }
        }

        Self { blocks }
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> BlockType {
        if y < 0 || y >= SNAP_Y as i32 {
            return BlockType::Air;
        }
        let sx = x + PAD as i32;
        let sz = z + PAD as i32;
        if sx < 0 || sx >= SNAP_X as i32 || sz < 0 || sz >= SNAP_Z as i32 {
            return BlockType::Air;
        }
        self.blocks[Self::index(x, y, z)]
    }
}

// --- Face definitions with known-correct winding order ---

struct FaceDef {
    normal: [f32; 3],
    vertices: [[f32; 3]; 4],
    neighbor_offset: [i32; 3],
    /// For each vertex, the 3 AO neighbor offsets [side1, side2, corner].
    ao_dirs: [[[i32; 3]; 3]; 4],
}

const FACES: [FaceDef; 6] = [
    // Top (+Y)
    FaceDef {
        normal: [0.0, 1.0, 0.0],
        vertices: [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]],
        neighbor_offset: [0, 1, 0],
        ao_dirs: [
            [[-1, 1, 0], [0, 1, -1], [-1, 1, -1]],
            [[1, 1, 0], [0, 1, -1], [1, 1, -1]],
            [[1, 1, 0], [0, 1, 1], [1, 1, 1]],
            [[-1, 1, 0], [0, 1, 1], [-1, 1, 1]],
        ],
    },
    // Bottom (-Y)
    FaceDef {
        normal: [0.0, -1.0, 0.0],
        vertices: [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        neighbor_offset: [0, -1, 0],
        ao_dirs: [
            [[-1, -1, 0], [0, -1, 1], [-1, -1, 1]],
            [[1, -1, 0], [0, -1, 1], [1, -1, 1]],
            [[1, -1, 0], [0, -1, -1], [1, -1, -1]],
            [[-1, -1, 0], [0, -1, -1], [-1, -1, -1]],
        ],
    },
    // North (+Z)
    FaceDef {
        normal: [0.0, 0.0, 1.0],
        vertices: [[1.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
        neighbor_offset: [0, 0, 1],
        ao_dirs: [
            [[1, 0, 1], [0, -1, 1], [1, -1, 1]],
            [[-1, 0, 1], [0, -1, 1], [-1, -1, 1]],
            [[-1, 0, 1], [0, 1, 1], [-1, 1, 1]],
            [[1, 0, 1], [0, 1, 1], [1, 1, 1]],
        ],
    },
    // South (-Z)
    FaceDef {
        normal: [0.0, 0.0, -1.0],
        vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        neighbor_offset: [0, 0, -1],
        ao_dirs: [
            [[-1, 0, -1], [0, -1, -1], [-1, -1, -1]],
            [[1, 0, -1], [0, -1, -1], [1, -1, -1]],
            [[1, 0, -1], [0, 1, -1], [1, 1, -1]],
            [[-1, 0, -1], [0, 1, -1], [-1, 1, -1]],
        ],
    },
    // East (+X)
    FaceDef {
        normal: [1.0, 0.0, 0.0],
        vertices: [[1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
        neighbor_offset: [1, 0, 0],
        ao_dirs: [
            [[1, 0, -1], [1, -1, 0], [1, -1, -1]],
            [[1, 0, 1], [1, -1, 0], [1, -1, 1]],
            [[1, 0, 1], [1, 1, 0], [1, 1, 1]],
            [[1, 0, -1], [1, 1, 0], [1, 1, -1]],
        ],
    },
    // West (-X)
    FaceDef {
        normal: [-1.0, 0.0, 0.0],
        vertices: [[0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 1.0]],
        neighbor_offset: [-1, 0, 0],
        ao_dirs: [
            [[-1, 0, 1], [-1, -1, 0], [-1, -1, 1]],
            [[-1, 0, -1], [-1, -1, 0], [-1, -1, -1]],
            [[-1, 0, -1], [-1, 1, 0], [-1, 1, -1]],
            [[-1, 0, 1], [-1, 1, 0], [-1, 1, 1]],
        ],
    },
];

/// Build a chunk mesh from a snapshot (per-face with AO, no greedy merging).
pub fn build_chunk_mesh_from_snapshot(snap: &ChunkSnapshot) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for y in 0..CHUNK_HEIGHT {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let lx = x as i32;
                let ly = y as i32;
                let lz = z as i32;

                let block = snap.get_block(lx, ly, lz);
                if !block.is_solid() {
                    continue;
                }

                let block_color = block.color().to_linear();
                let base_color = [
                    block_color.red,
                    block_color.green,
                    block_color.blue,
                    block_color.alpha,
                ];

                for face in &FACES {
                    let nx = lx + face.neighbor_offset[0];
                    let ny = ly + face.neighbor_offset[1];
                    let nz = lz + face.neighbor_offset[2];

                    let neighbor = snap.get_block(nx, ny, nz);
                    if !neighbor.is_transparent() {
                        continue;
                    }

                    // Compute AO per vertex
                    let mut ao = [0u8; 4];
                    for (i, ao_nb) in face.ao_dirs.iter().enumerate() {
                        let s1 = snap.get_block(lx + ao_nb[0][0], ly + ao_nb[0][1], lz + ao_nb[0][2]).is_solid();
                        let s2 = snap.get_block(lx + ao_nb[1][0], ly + ao_nb[1][1], lz + ao_nb[1][2]).is_solid();
                        let corner = snap.get_block(lx + ao_nb[2][0], ly + ao_nb[2][1], lz + ao_nb[2][2]).is_solid();
                        ao[i] = vertex_ao(s1, s2, corner);
                    }

                    let base_index = positions.len() as u32;

                    for (i, vertex) in face.vertices.iter().enumerate() {
                        positions.push([
                            vertex[0] + x as f32,
                            vertex[1] + y as f32,
                            vertex[2] + z as f32,
                        ]);
                        normals.push(face.normal);
                        let brightness = AO_CURVE[ao[i] as usize];
                        colors.push([
                            base_color[0] * brightness,
                            base_color[1] * brightness,
                            base_color[2] * brightness,
                            base_color[3],
                        ]);
                    }

                    // Flip quad diagonal for AO anisotropy fix
                    if ao[0] + ao[2] <= ao[1] + ao[3] {
                        indices.push(base_index);
                        indices.push(base_index + 1);
                        indices.push(base_index + 2);
                        indices.push(base_index);
                        indices.push(base_index + 2);
                        indices.push(base_index + 3);
                    } else {
                        indices.push(base_index + 1);
                        indices.push(base_index + 2);
                        indices.push(base_index + 3);
                        indices.push(base_index + 1);
                        indices.push(base_index + 3);
                        indices.push(base_index);
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Convenience wrapper for synchronous meshing.
pub fn build_chunk_mesh(chunk_pos: ChunkPos, chunk_map: &ChunkMap) -> Mesh {
    let snap = ChunkSnapshot::from_chunk_map(chunk_pos, chunk_map);
    build_chunk_mesh_from_snapshot(&snap)
}

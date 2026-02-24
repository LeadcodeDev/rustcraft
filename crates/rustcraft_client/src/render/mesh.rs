use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::world::block::BlockColor;
use crate::world::chunk::{CHUNK_HEIGHT, CHUNK_SIZE, ChunkMap, ChunkPos};

struct FaceDef {
    normal: [f32; 3],
    vertices: [[f32; 3]; 4],
    neighbor_offset: [i32; 3],
}

const FACES: [FaceDef; 6] = [
    // Top (+Y)
    FaceDef {
        normal: [0.0, 1.0, 0.0],
        vertices: [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        neighbor_offset: [0, 1, 0],
    },
    // Bottom (-Y)
    FaceDef {
        normal: [0.0, -1.0, 0.0],
        vertices: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        neighbor_offset: [0, -1, 0],
    },
    // North (+Z)
    FaceDef {
        normal: [0.0, 0.0, 1.0],
        vertices: [
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ],
        neighbor_offset: [0, 0, 1],
    },
    // South (-Z)
    FaceDef {
        normal: [0.0, 0.0, -1.0],
        vertices: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        neighbor_offset: [0, 0, -1],
    },
    // East (+X)
    FaceDef {
        normal: [1.0, 0.0, 0.0],
        vertices: [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
        ],
        neighbor_offset: [1, 0, 0],
    },
    // West (-X)
    FaceDef {
        normal: [-1.0, 0.0, 0.0],
        vertices: [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
        ],
        neighbor_offset: [-1, 0, 0],
    },
];

pub fn build_chunk_mesh(chunk_pos: ChunkPos, chunk_map: &ChunkMap) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let world_offset_x = chunk_pos.0 * CHUNK_SIZE as i32;
    let world_offset_z = chunk_pos.1 * CHUNK_SIZE as i32;

    for y in 0..CHUNK_HEIGHT {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let wx = world_offset_x + x as i32;
                let wy = y as i32;
                let wz = world_offset_z + z as i32;

                let block = chunk_map.get_block(wx, wy, wz);
                if !block.is_solid() {
                    continue;
                }

                let block_color = block.color().to_linear();
                let color_arr = [
                    block_color.red,
                    block_color.green,
                    block_color.blue,
                    block_color.alpha,
                ];

                for face in &FACES {
                    let nx = wx + face.neighbor_offset[0];
                    let ny = wy + face.neighbor_offset[1];
                    let nz = wz + face.neighbor_offset[2];

                    let neighbor = chunk_map.get_block(nx, ny, nz);
                    if !neighbor.is_transparent() {
                        continue;
                    }

                    let base_index = positions.len() as u32;

                    for vertex in &face.vertices {
                        positions.push([
                            vertex[0] + x as f32,
                            vertex[1] + y as f32,
                            vertex[2] + z as f32,
                        ]);
                        normals.push(face.normal);
                        colors.push(color_arr);
                    }

                    indices.push(base_index);
                    indices.push(base_index + 1);
                    indices.push(base_index + 2);
                    indices.push(base_index);
                    indices.push(base_index + 2);
                    indices.push(base_index + 3);
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

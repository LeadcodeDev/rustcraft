pub mod mesh;

use bevy::prelude::*;

use crate::world::chunk::{CHUNK_SIZE, ChunkMap, ChunkPos};
use mesh::build_chunk_mesh;

#[derive(Component)]
pub struct ChunkEntity(pub ChunkPos);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            spawn_chunk_meshes.after(crate::world::generation::generate_world),
        )
        .add_systems(Update, remesh_dirty_chunks);
    }
}

fn spawn_chunk_meshes(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        cull_mode: None,
        ..default()
    });

    for (&chunk_pos, _chunk) in &chunk_map.chunks {
        let mesh = build_chunk_mesh(chunk_pos, &chunk_map);
        let mesh_handle = meshes.add(mesh);

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(
                (chunk_pos.0 * CHUNK_SIZE as i32) as f32,
                0.0,
                (chunk_pos.1 * CHUNK_SIZE as i32) as f32,
            ),
            ChunkEntity(chunk_pos),
        ));
    }
}

fn remesh_dirty_chunks(
    mut chunk_map: ResMut<ChunkMap>,
    query: Query<(&ChunkEntity, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dirty_positions: Vec<ChunkPos> = chunk_map
        .chunks
        .iter()
        .filter(|(_, c)| c.dirty)
        .map(|(&pos, _)| pos)
        .collect();

    if dirty_positions.is_empty() {
        return;
    }

    for &chunk_pos in &dirty_positions {
        let new_mesh = build_chunk_mesh(chunk_pos, &chunk_map);

        for (chunk_entity, mesh3d) in &query {
            if chunk_entity.0 == chunk_pos {
                if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
                    *mesh = new_mesh;
                    break;
                }
            }
        }
    }

    for pos in dirty_positions {
        if let Some(chunk) = chunk_map.chunks.get_mut(&pos) {
            chunk.dirty = false;
        }
    }
}

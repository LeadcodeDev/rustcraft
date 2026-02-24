pub mod mesh;

use std::collections::HashSet;

use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use bevy::tasks::{Task, block_on, ComputeTaskPool};

use crate::app_state::AppState;
use crate::player::camera::FlyCam;
use crate::world::chunk::{CHUNK_SIZE, ChunkMap, ChunkPos};
use mesh::{ChunkSnapshot, build_chunk_mesh, build_chunk_mesh_from_snapshot};

/// Maximum number of chunk mesh tasks to dispatch per frame.
const MAX_CHUNK_DISPATCHES_PER_FRAME: usize = 4;

/// Maximum number of chunk remeshes per frame (dirty chunk updates).
const MAX_CHUNK_REMESHES_PER_FRAME: usize = 4;

/// Cosine of the half-angle for chunk visibility culling.
/// ~140° total cone → cos(70°) ≈ 0.34. Chunks behind the player are hidden.
const VISIBILITY_COS_HALF_ANGLE: f32 = 0.34;

#[derive(Component)]
pub struct ChunkEntity(pub ChunkPos);

/// Tracks which chunks have been spawned as Bevy entities.
#[derive(Resource, Default)]
pub struct SpawnedChunks(pub HashSet<ChunkPos>);

/// Shared material for all chunk meshes (vertex colors provide per-block coloring).
#[derive(Resource)]
pub struct ChunkMaterial(pub Handle<StandardMaterial>);

/// Pending async mesh build tasks.
#[derive(Resource, Default)]
pub struct PendingChunkMeshes {
    tasks: Vec<(ChunkPos, Task<Mesh>)>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnedChunks>()
            .init_resource::<PendingChunkMeshes>()
            .add_systems(OnEnter(AppState::InGame), setup_chunk_material)
            .add_systems(
                Update,
                (
                    dispatch_chunk_mesh_tasks,
                    collect_chunk_mesh_tasks,
                    remesh_dirty_chunks,
                    despawn_unloaded_chunks,
                    update_chunk_visibility,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn setup_chunk_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.insert_resource(ChunkMaterial(handle));
}

/// Returns the XZ center of a chunk in world space.
fn chunk_center_xz(chunk_pos: ChunkPos) -> Vec2 {
    Vec2::new(
        chunk_pos.0 as f32 * CHUNK_SIZE as f32 + CHUNK_SIZE as f32 / 2.0,
        chunk_pos.1 as f32 * CHUNK_SIZE as f32 + CHUNK_SIZE as f32 / 2.0,
    )
}

/// Returns true if a chunk is within the camera's field of view cone (XZ plane).
fn is_chunk_in_fov(chunk_pos: ChunkPos, cam_pos_xz: Vec2, cam_forward_xz: Vec2) -> bool {
    let chunk_center = chunk_center_xz(chunk_pos);
    let to_chunk = chunk_center - cam_pos_xz;
    let dist = to_chunk.length();

    // Always show chunks the player is standing on or very close to
    if dist < CHUNK_SIZE as f32 * 2.0 {
        return true;
    }

    let dir = to_chunk / dist;
    let dot = cam_forward_xz.dot(dir);
    dot >= VISIBILITY_COS_HALF_ANGLE
}

/// Dispatches async mesh building tasks for unspawned chunks.
/// Creates snapshots on the main thread, then spawns compute tasks.
fn dispatch_chunk_mesh_tasks(
    chunk_map: Res<ChunkMap>,
    spawned: Res<SpawnedChunks>,
    mut pending: ResMut<PendingChunkMeshes>,
    camera_query: Query<&Transform, With<FlyCam>>,
) {
    // Don't dispatch if we already have many pending tasks
    if pending.tasks.len() >= MAX_CHUNK_DISPATCHES_PER_FRAME * 2 {
        return;
    }

    // Collect chunks that need meshing (not spawned, not already pending)
    let pending_positions: HashSet<ChunkPos> = pending.tasks.iter().map(|(pos, _)| *pos).collect();
    let unspawned: Vec<ChunkPos> = chunk_map
        .chunks
        .keys()
        .copied()
        .filter(|pos| !spawned.0.contains(pos) && !pending_positions.contains(pos))
        .collect();

    if unspawned.is_empty() {
        return;
    }

    // Get camera info for FOV prioritization
    let (cam_pos_xz, cam_forward_xz) = if let Ok(cam_transform) = camera_query.get_single() {
        let pos = cam_transform.translation;
        let fwd = cam_transform.forward().as_vec3();
        (
            Vec2::new(pos.x, pos.z),
            Vec2::new(fwd.x, fwd.z).normalize_or_zero(),
        )
    } else {
        (Vec2::ZERO, Vec2::new(0.0, -1.0))
    };

    // Sort: FOV chunks first, then by distance to camera
    let mut sorted = unspawned;
    sorted.sort_by(|a, b| {
        let a_in_fov = is_chunk_in_fov(*a, cam_pos_xz, cam_forward_xz);
        let b_in_fov = is_chunk_in_fov(*b, cam_pos_xz, cam_forward_xz);

        match (a_in_fov, b_in_fov) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_dist = chunk_center_xz(*a).distance_squared(cam_pos_xz);
                let b_dist = chunk_center_xz(*b).distance_squared(cam_pos_xz);
                a_dist
                    .partial_cmp(&b_dist)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });

    let pool = ComputeTaskPool::get();
    let mut dispatched = 0;

    for chunk_pos in sorted {
        // Create snapshot on main thread (fast memcpy)
        let snap = ChunkSnapshot::from_chunk_map(chunk_pos, &chunk_map);

        // Dispatch mesh building to compute thread pool
        let task = pool.spawn(async move { build_chunk_mesh_from_snapshot(&snap) });

        pending.tasks.push((chunk_pos, task));

        dispatched += 1;
        if dispatched >= MAX_CHUNK_DISPATCHES_PER_FRAME {
            break;
        }
    }
}

/// Collects completed mesh tasks and spawns chunk entities.
fn collect_chunk_mesh_tasks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    mut spawned: ResMut<SpawnedChunks>,
    mut pending: ResMut<PendingChunkMeshes>,
    chunk_map: Res<ChunkMap>,
) {
    let mut remaining = Vec::new();

    for (chunk_pos, mut task) in pending.tasks.drain(..) {
        // Skip if chunk was removed from ChunkMap while we were building
        if !chunk_map.chunks.contains_key(&chunk_pos) {
            continue;
        }

        // Skip if already spawned (e.g. by remesh)
        if spawned.0.contains(&chunk_pos) {
            continue;
        }

        if task.is_finished() {
            let mesh = block_on(&mut task);
            let mesh_handle = meshes.add(mesh);

            commands.spawn((
                StateScoped(AppState::InGame),
                Mesh3d(mesh_handle),
                MeshMaterial3d(chunk_material.0.clone()),
                Transform::from_xyz(
                    (chunk_pos.0 * CHUNK_SIZE as i32) as f32,
                    0.0,
                    (chunk_pos.1 * CHUNK_SIZE as i32) as f32,
                ),
                ChunkEntity(chunk_pos),
            ));

            spawned.0.insert(chunk_pos);
        } else {
            remaining.push((chunk_pos, task));
        }
    }

    pending.tasks = remaining;
}

fn remesh_dirty_chunks(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    query: Query<(Entity, &ChunkEntity, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dirty_positions: Vec<ChunkPos> = chunk_map
        .chunks
        .iter()
        .filter(|(_, c)| c.dirty)
        .map(|(&pos, _)| pos)
        .take(MAX_CHUNK_REMESHES_PER_FRAME)
        .collect();

    if dirty_positions.is_empty() {
        return;
    }

    for &chunk_pos in &dirty_positions {
        let new_mesh = build_chunk_mesh(chunk_pos, &chunk_map);

        for (entity, chunk_entity, mesh3d) in &query {
            if chunk_entity.0 == chunk_pos {
                if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
                    *mesh = new_mesh;
                    if let Some(aabb) = mesh.compute_aabb() {
                        commands.entity(entity).insert(aabb);
                    }
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

/// Despawns chunk entities whose data has been removed from the ChunkMap.
/// Explicitly removes mesh assets to avoid GC lag.
fn despawn_unloaded_chunks(
    mut commands: Commands,
    chunk_map: Res<ChunkMap>,
    mut spawned: ResMut<SpawnedChunks>,
    query: Query<(Entity, &ChunkEntity, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, chunk_entity, mesh3d) in &query {
        if !chunk_map.chunks.contains_key(&chunk_entity.0) {
            meshes.remove(&mesh3d.0);
            commands.entity(entity).despawn();
            spawned.0.remove(&chunk_entity.0);
        }
    }
}

/// Toggles chunk entity visibility based on the camera's FOV cone.
/// Data stays in ChunkMap so neighbor meshes remain correct at borders.
/// True eviction is handled server-side via `ChunkUnload` when the player
/// moves beyond VIEW_DISTANCE.
fn update_chunk_visibility(
    camera_query: Query<&Transform, With<FlyCam>>,
    mut query: Query<(&ChunkEntity, &mut Visibility)>,
) {
    let Ok(cam_transform) = camera_query.get_single() else {
        return;
    };

    let pos = cam_transform.translation;
    let fwd = cam_transform.forward().as_vec3();
    let cam_pos_xz = Vec2::new(pos.x, pos.z);
    let cam_forward_xz = Vec2::new(fwd.x, fwd.z).normalize_or_zero();

    for (chunk_entity, mut visibility) in &mut query {
        let in_fov = is_chunk_in_fov(chunk_entity.0, cam_pos_xz, cam_forward_xz);
        let new_vis = if in_fov {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != new_vis {
            *visibility = new_vis;
        }
    }
}

use bevy::prelude::*;

use crate::network::{ServerDroppedItemRemoveEvent, ServerDroppedItemSpawnEvent};
use crate::world::block::BlockColor;

const DROPPED_ITEM_SCALE: f32 = 0.3;
const ROTATION_SPEED: f32 = 1.5;

/// Links a client entity to a server-managed dropped item.
#[derive(Component)]
pub struct ServerDroppedItem {
    pub id: u64,
}

pub struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ServerDroppedItemSpawnEvent>()
            .add_event::<ServerDroppedItemRemoveEvent>()
            .add_systems(
                Update,
                (
                    handle_dropped_item_spawn,
                    handle_dropped_item_remove,
                    dropped_item_rotation,
                ),
            );
    }
}

fn handle_dropped_item_spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_spawn: EventReader<ServerDroppedItemSpawnEvent>,
) {
    for event in ev_spawn.read() {
        let cube_count = match event.count {
            0 => continue,
            1 => 1,
            2..=31 => 2,
            _ => 3,
        };

        let mesh = meshes.add(Cuboid::new(
            DROPPED_ITEM_SCALE,
            DROPPED_ITEM_SCALE,
            DROPPED_ITEM_SCALE,
        ));
        let material = materials.add(StandardMaterial {
            base_color: event.block.color(),
            ..default()
        });

        let mut parent = commands.spawn((
            ServerDroppedItem { id: event.id },
            Transform::from_translation(event.position),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        parent.with_children(|children| {
            for i in 0..cube_count {
                let offset = match i {
                    0 => Vec3::ZERO,
                    1 => Vec3::new(0.08, 0.05, 0.06),
                    2 => Vec3::new(-0.06, 0.10, -0.04),
                    _ => Vec3::ZERO,
                };
                children.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(offset),
                ));
            }
        });
    }
}

fn handle_dropped_item_remove(
    mut commands: Commands,
    mut ev_remove: EventReader<ServerDroppedItemRemoveEvent>,
    query: Query<(Entity, &ServerDroppedItem)>,
) {
    for event in ev_remove.read() {
        for (entity, item) in &query {
            if item.id == event.id {
                commands.entity(entity).despawn_recursive();
                break;
            }
        }
    }
}

fn dropped_item_rotation(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<ServerDroppedItem>>,
) {
    let dt = time.delta_secs();
    for mut transform in &mut query {
        transform.rotate_y(ROTATION_SPEED * dt);
    }
}

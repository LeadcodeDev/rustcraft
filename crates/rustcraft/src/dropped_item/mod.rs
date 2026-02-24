use bevy::prelude::*;

use crate::events::{ItemDroppedToWorldEvent, ItemsCollectedEvent};
use crate::inventory::{Inventory, ItemStack};
use crate::player::camera::{FlyCam, Player};
use crate::world::chunk::ChunkMap;

const PICKUP_RADIUS: f32 = 2.0;
const PICKUP_DELAY: f32 = 1.5;
const COLLECT_SPEED: f32 = 8.0;
const COLLECT_ABSORB_DIST: f32 = 0.3;
const DROPPED_ITEM_SCALE: f32 = 0.3;
const GRAVITY: f32 = 32.0;
const TERMINAL_VELOCITY: f32 = 50.0;
const ROTATION_SPEED: f32 = 1.5;

#[derive(Component)]
pub struct DroppedItem {
    pub stack: ItemStack,
    pub velocity: Vec3,
    pub grounded: bool,
    pub age: f32,
    pub collecting: bool,
}

pub struct DroppedItemPlugin;

impl Plugin for DroppedItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_dropped_items,
                dropped_item_physics,
                dropped_item_rotation,
                pickup_dropped_items,
                collect_dropped_items,
            ),
        );
    }
}

fn spawn_dropped_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_drop: EventReader<ItemDroppedToWorldEvent>,
) {
    for event in ev_drop.read() {
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
            base_color: event.block_type.color(),
            ..default()
        });

        // Spawn parent entity with DroppedItem component
        let mut parent = commands.spawn((
            DroppedItem {
                stack: ItemStack::new(event.block_type, event.count),
                velocity: event.velocity,
                grounded: false,
                age: 0.0,
                collecting: false,
            },
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

fn dropped_item_physics(
    time: Res<Time>,
    chunk_map: Res<ChunkMap>,
    mut query: Query<(&mut DroppedItem, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut item, mut transform) in &mut query {
        if item.collecting {
            continue;
        }

        if item.grounded {
            // Apply friction then stop
            item.velocity.x *= (1.0 - 5.0 * dt).max(0.0);
            item.velocity.z *= (1.0 - 5.0 * dt).max(0.0);
            if item.velocity.x.abs() < 0.01 && item.velocity.z.abs() < 0.01 {
                item.velocity = Vec3::ZERO;
                continue;
            }

            // Slide horizontally only
            let new_x = transform.translation.x + item.velocity.x * dt;
            let by = (transform.translation.y - 0.01).floor() as i32;
            let bz = transform.translation.z.floor() as i32;
            if !chunk_map
                .get_block(new_x.floor() as i32, by + 1, bz)
                .is_solid()
            {
                transform.translation.x = new_x;
            } else {
                item.velocity.x = 0.0;
            }

            let new_z = transform.translation.z + item.velocity.z * dt;
            let bx = transform.translation.x.floor() as i32;
            if !chunk_map
                .get_block(bx, by + 1, new_z.floor() as i32)
                .is_solid()
            {
                transform.translation.z = new_z;
            } else {
                item.velocity.z = 0.0;
            }
            continue;
        }

        // Airborne: apply gravity
        item.velocity.y -= GRAVITY * dt;
        item.velocity.y = item.velocity.y.max(-TERMINAL_VELOCITY);

        // Move X
        let new_x = transform.translation.x + item.velocity.x * dt;
        let by = transform.translation.y.floor() as i32;
        let bz = transform.translation.z.floor() as i32;
        if chunk_map.get_block(new_x.floor() as i32, by, bz).is_solid() {
            item.velocity.x = 0.0;
        } else {
            transform.translation.x = new_x;
        }

        // Move Z
        let new_z = transform.translation.z + item.velocity.z * dt;
        let bx = transform.translation.x.floor() as i32;
        if chunk_map.get_block(bx, by, new_z.floor() as i32).is_solid() {
            item.velocity.z = 0.0;
        } else {
            transform.translation.z = new_z;
        }

        // Move Y
        let new_y = transform.translation.y + item.velocity.y * dt;
        let check_y = (new_y - 0.01).floor() as i32;
        let bx = transform.translation.x.floor() as i32;
        let bz = transform.translation.z.floor() as i32;

        if item.velocity.y <= 0.0 && chunk_map.get_block(bx, check_y, bz).is_solid() {
            transform.translation.y = (check_y + 1) as f32 + DROPPED_ITEM_SCALE / 2.0 + 0.02;
            item.velocity.y = 0.0;
            item.grounded = true;
        } else {
            transform.translation.y = new_y;
        }
    }
}

fn dropped_item_rotation(time: Res<Time>, mut query: Query<&mut Transform, With<DroppedItem>>) {
    let dt = time.delta_secs();
    for mut transform in &mut query {
        transform.rotate_y(ROTATION_SPEED * dt);
    }
}

fn pickup_dropped_items(
    time: Res<Time>,
    inventory: Res<Inventory>,
    player_query: Query<&Player, With<FlyCam>>,
    mut item_query: Query<(Entity, &mut DroppedItem, &Transform)>,
) {
    let Ok(player) = player_query.get_single() else {
        return;
    };

    let dt = time.delta_secs();

    for (_entity, mut item, transform) in &mut item_query {
        item.age += dt;
        if item.collecting || item.age < PICKUP_DELAY {
            continue;
        }

        let distance = player.position.distance(transform.translation);
        if distance > PICKUP_RADIUS {
            continue;
        }

        // Check inventory has room before starting collection
        if inventory.find_slot_for(item.stack.block).is_some() {
            item.collecting = true;
        }
    }
}

fn collect_dropped_items(
    mut commands: Commands,
    time: Res<Time>,
    mut inventory: ResMut<Inventory>,
    player_query: Query<(&Transform, &Player), With<FlyCam>>,
    mut item_query: Query<(Entity, &mut DroppedItem, &mut Transform), Without<FlyCam>>,
    mut ev_collected: EventWriter<ItemsCollectedEvent>,
) {
    let Ok((player_transform, player)) = player_query.get_single() else {
        return;
    };

    let dt = time.delta_secs();
    let target = player.position + Vec3::Y * 0.5;
    let mut collected: Vec<ItemStack> = Vec::new();

    for (entity, mut item, mut transform) in &mut item_query {
        if !item.collecting {
            continue;
        }

        let dir = (target - transform.translation).normalize_or_zero();
        transform.translation += dir * COLLECT_SPEED * dt;

        let distance = transform.translation.distance(target);
        if distance < COLLECT_ABSORB_DIST {
            let added = item.stack.count;
            let leftover = inventory.add_stack(item.stack.block, item.stack.count);
            if leftover == 0 {
                collected.push(ItemStack::new(item.stack.block, added));
                commands.entity(entity).despawn_recursive();
            } else {
                let actually_added = added - leftover;
                if actually_added > 0 {
                    collected.push(ItemStack::new(item.stack.block, actually_added));
                }
                item.stack.count = leftover;
                item.collecting = false;
            }
        }
    }

    if !collected.is_empty() {
        ev_collected.send(ItemsCollectedEvent {
            items: collected,
            player: player.location(player_transform),
        });
    }
}

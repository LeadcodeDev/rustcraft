use bevy::prelude::*;

use crate::player::camera::GameMode;
use crate::world::block::BlockType;

#[derive(Event)]
pub struct BlockPlaced {
    pub position: IVec3,
    pub block_type: BlockType,
}

#[derive(Event)]
pub struct BlockRemoved {
    pub position: IVec3,
    pub block_type: BlockType,
}

#[derive(Event)]
pub struct PlayerMoved {
    pub old_position: Vec3,
    pub new_position: Vec3,
}

#[derive(Event)]
pub struct GameModeChanged {
    pub new_mode: GameMode,
}

#[derive(Event)]
pub struct InventoryPickedUp {
    pub slot: usize,
    pub block_type: BlockType,
    pub count: u32,
}

#[derive(Event)]
pub struct InventoryDropped {
    pub from_slot: usize,
    pub to_slot: usize,
    pub block_type: BlockType,
    pub count: u32,
}

#[derive(Event)]
pub struct ItemDroppedToWorld {
    pub block_type: BlockType,
    pub count: u32,
    pub position: Vec3,
    pub velocity: Vec3,
}

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BlockPlaced>()
            .add_event::<BlockRemoved>()
            .add_event::<PlayerMoved>()
            .add_event::<GameModeChanged>()
            .add_event::<InventoryPickedUp>()
            .add_event::<InventoryDropped>()
            .add_event::<ItemDroppedToWorld>()
            .add_systems(
                Update,
                (
                    log_block_placed,
                    log_block_removed,
                    log_player_moved,
                    log_gamemode_changed,
                    log_inventory_picked_up,
                    log_inventory_dropped,
                    log_item_dropped_to_world,
                ),
            );
    }
}

fn log_block_placed(mut reader: EventReader<BlockPlaced>) {
    for event in reader.read() {
        info!(
            "[BlockPlaced] {:?} at ({}, {}, {})",
            event.block_type, event.position.x, event.position.y, event.position.z
        );
    }
}

fn log_block_removed(mut reader: EventReader<BlockRemoved>) {
    for event in reader.read() {
        info!(
            "[BlockRemoved] {:?} at ({}, {}, {})",
            event.block_type, event.position.x, event.position.y, event.position.z
        );
    }
}

fn log_player_moved(mut reader: EventReader<PlayerMoved>) {
    for event in reader.read() {
        info!(
            "[PlayerMoved] ({:.1}, {:.1}, {:.1}) -> ({:.1}, {:.1}, {:.1})",
            event.old_position.x,
            event.old_position.y,
            event.old_position.z,
            event.new_position.x,
            event.new_position.y,
            event.new_position.z
        );
    }
}

fn log_gamemode_changed(mut reader: EventReader<GameModeChanged>) {
    for event in reader.read() {
        info!("[GameModeChanged] -> {:?}", event.new_mode);
    }
}

fn log_inventory_picked_up(mut reader: EventReader<InventoryPickedUp>) {
    for event in reader.read() {
        info!(
            "[InventoryPickedUp] {:?} x{} from slot {}",
            event.block_type, event.count, event.slot
        );
    }
}

fn log_inventory_dropped(mut reader: EventReader<InventoryDropped>) {
    for event in reader.read() {
        info!(
            "[InventoryDropped] {:?} x{} from slot {} to slot {}",
            event.block_type, event.count, event.from_slot, event.to_slot
        );
    }
}

fn log_item_dropped_to_world(mut reader: EventReader<ItemDroppedToWorld>) {
    for event in reader.read() {
        info!(
            "[ItemDroppedToWorld] {:?} x{} at ({:.1}, {:.1}, {:.1})",
            event.block_type, event.count, event.position.x, event.position.y, event.position.z
        );
    }
}

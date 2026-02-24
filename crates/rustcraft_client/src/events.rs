use bevy::prelude::*;

use crate::inventory::ItemStack;
use crate::player::camera::{GameMode, Location};
use crate::world::block::BlockType;

// --- Events ---

#[derive(Event)]
pub struct BlockPlacedEvent {
    pub position: IVec3,
    pub block_type: BlockType,
    pub player: Location,
}

#[derive(Event)]
pub struct BlockRemovedEvent {
    pub position: IVec3,
    pub block_type: BlockType,
    pub player: Location,
}

#[derive(Event)]
pub struct PlayerMovedEvent {
    pub old_position: Vec3,
    pub new_position: Vec3,
    pub player: Location,
}

#[derive(Event)]
pub struct GameModeChangedEvent {
    pub new_mode: GameMode,
    pub player: Location,
}

#[derive(Event)]
pub struct InventoryPickedUpEvent {
    pub slot: usize,
    pub block_type: BlockType,
    pub count: u32,
    pub player: Location,
}

#[derive(Event)]
pub struct InventoryDroppedEvent {
    pub from_slot: usize,
    pub to_slot: usize,
    pub block_type: BlockType,
    pub count: u32,
    pub player: Location,
}

#[derive(Event)]
pub struct ItemDroppedToWorldEvent {
    pub block_type: BlockType,
    pub count: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub player: Location,
}

#[derive(Event)]
pub struct ItemsCollectedEvent {
    pub items: Vec<ItemStack>,
    pub player: Location,
}

#[derive(Event)]
pub struct PlayerJoinEvent {
    pub player_id: u64,
    pub name: String,
    pub position: Vec3,
}

#[derive(Event)]
pub struct PlayerLeaveEvent {
    pub player_id: u64,
}

// --- Plugin trait ---

#[allow(unused_variables)]
pub trait RustcraftPlugin: Send + Sync + 'static {
    fn on_block_placed(&self, event: &BlockPlacedEvent) {}
    fn on_block_removed(&self, event: &BlockRemovedEvent) {}
    fn on_player_moved(&self, event: &PlayerMovedEvent) {}
    fn on_gamemode_changed(&self, event: &GameModeChangedEvent) {}
    fn on_inventory_picked_up(&self, event: &InventoryPickedUpEvent) {}
    fn on_inventory_dropped(&self, event: &InventoryDroppedEvent) {}
    fn on_item_dropped_to_world(&self, event: &ItemDroppedToWorldEvent) {}
    fn on_items_collected(&self, event: &ItemsCollectedEvent) {}
    fn on_player_join(&self, event: &PlayerJoinEvent) {}
    fn on_player_leave(&self, event: &PlayerLeaveEvent) {}
}

// --- Registry ---

#[derive(Resource)]
struct PluginRegistry {
    plugins: Vec<Box<dyn RustcraftPlugin>>,
}

// --- Dispatch systems ---

fn dispatch_block_placed(
    mut reader: EventReader<BlockPlacedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_block_placed(event);
        }
    }
}

fn dispatch_block_removed(
    mut reader: EventReader<BlockRemovedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_block_removed(event);
        }
    }
}

fn dispatch_player_moved(
    mut reader: EventReader<PlayerMovedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_player_moved(event);
        }
    }
}

fn dispatch_gamemode_changed(
    mut reader: EventReader<GameModeChangedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_gamemode_changed(event);
        }
    }
}

fn dispatch_inventory_picked_up(
    mut reader: EventReader<InventoryPickedUpEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_inventory_picked_up(event);
        }
    }
}

fn dispatch_inventory_dropped(
    mut reader: EventReader<InventoryDroppedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_inventory_dropped(event);
        }
    }
}

fn dispatch_item_dropped_to_world(
    mut reader: EventReader<ItemDroppedToWorldEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_item_dropped_to_world(event);
        }
    }
}

fn dispatch_items_collected(
    mut reader: EventReader<ItemsCollectedEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_items_collected(event);
        }
    }
}

fn dispatch_player_join(
    mut reader: EventReader<PlayerJoinEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_player_join(event);
        }
    }
}

fn dispatch_player_leave(
    mut reader: EventReader<PlayerLeaveEvent>,
    registry: Res<PluginRegistry>,
) {
    for event in reader.read() {
        for plugin in &registry.plugins {
            plugin.on_player_leave(event);
        }
    }
}

// --- EventsPlugin builder ---

pub struct EventsPlugin {
    plugins: std::sync::Mutex<Vec<Box<dyn RustcraftPlugin>>>,
}

impl EventsPlugin {
    pub fn new() -> Self {
        Self {
            plugins: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn new_with(plugins: Vec<Box<dyn RustcraftPlugin>>) -> Self {
        Self {
            plugins: std::sync::Mutex::new(plugins),
        }
    }

    pub fn add_plugin(self, plugin: impl RustcraftPlugin) -> Self {
        self.plugins.lock().unwrap().push(Box::new(plugin));
        self
    }
}

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        let plugins = self.plugins.lock().unwrap().drain(..).collect();
        app.insert_resource(PluginRegistry { plugins });

        app.add_event::<BlockPlacedEvent>()
            .add_event::<BlockRemovedEvent>()
            .add_event::<PlayerMovedEvent>()
            .add_event::<GameModeChangedEvent>()
            .add_event::<InventoryPickedUpEvent>()
            .add_event::<InventoryDroppedEvent>()
            .add_event::<ItemDroppedToWorldEvent>()
            .add_event::<ItemsCollectedEvent>()
            .add_event::<PlayerJoinEvent>()
            .add_event::<PlayerLeaveEvent>()
            .add_systems(
                Update,
                (
                    dispatch_block_placed,
                    dispatch_block_removed,
                    dispatch_player_moved,
                    dispatch_gamemode_changed,
                    dispatch_inventory_picked_up,
                    dispatch_inventory_dropped,
                    dispatch_item_dropped_to_world,
                    dispatch_items_collected,
                    dispatch_player_join,
                    dispatch_player_leave,
                ),
            );
    }
}

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::player::camera::GameState;
use crate::world::block::BlockType;

pub const MAX_STACK: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub block: BlockType,
    pub count: u32,
}

impl ItemStack {
    pub fn new(block: BlockType, count: u32) -> Self {
        Self {
            block,
            count: count.min(MAX_STACK),
        }
    }
}

#[derive(Resource)]
pub struct Inventory {
    pub slots: [Option<ItemStack>; 36],
    pub active_slot: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        let mut slots = [None; 36];
        slots[0] = Some(ItemStack::new(BlockType::Grass, 64));
        slots[1] = Some(ItemStack::new(BlockType::Dirt, 64));
        slots[2] = Some(ItemStack::new(BlockType::Stone, 64));
        slots[3] = Some(ItemStack::new(BlockType::Sand, 64));
        slots[4] = Some(ItemStack::new(BlockType::Wood, 64));
        slots[5] = Some(ItemStack::new(BlockType::Leaves, 64));
        slots[6] = Some(ItemStack::new(BlockType::Water, 64));
        Self {
            slots,
            active_slot: 0,
        }
    }
}

impl Inventory {
    pub fn active_block(&self) -> Option<BlockType> {
        self.slots[self.active_slot].map(|stack| stack.block)
    }

    /// Decrement the active slot count by 1. Removes the stack if count reaches 0.
    pub fn consume_active(&mut self) {
        if let Some(stack) = &mut self.slots[self.active_slot] {
            stack.count -= 1;
            if stack.count == 0 {
                self.slots[self.active_slot] = None;
            }
        }
    }

    /// Find the first slot that can accept this block type.
    /// Priority: same type with room, then empty slot. Hotbar first (0..9), then inventory (9..36).
    pub fn find_slot_for(&self, block: BlockType) -> Option<usize> {
        let order: Vec<usize> = (0..9).chain(9..36).collect();
        // First pass: slot with same type and room
        for &i in &order {
            if let Some(stack) = &self.slots[i] {
                if stack.block == block && stack.count < MAX_STACK {
                    return Some(i);
                }
            }
        }
        // Second pass: first empty slot
        for &i in &order {
            if self.slots[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    /// Try to add an ItemStack to the inventory. Returns leftover count (0 if fully added).
    pub fn add_stack(&mut self, block: BlockType, mut count: u32) -> u32 {
        while count > 0 {
            if let Some(slot_idx) = self.find_slot_for(block) {
                if let Some(stack) = &mut self.slots[slot_idx] {
                    let space = MAX_STACK - stack.count;
                    let add = count.min(space);
                    stack.count += add;
                    count -= add;
                } else {
                    let add = count.min(MAX_STACK);
                    self.slots[slot_idx] = Some(ItemStack::new(block, add));
                    count -= add;
                }
            } else {
                break;
            }
        }
        count
    }
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .add_systems(Update, scroll_hotbar);
    }
}

fn scroll_hotbar(
    game_state: Res<GameState>,
    mut mouse_wheel: EventReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<Inventory>,
) {
    if *game_state != GameState::Playing {
        return;
    }

    for event in mouse_wheel.read() {
        // macOS natural scrolling: delta.y > 0 = next slot, delta.y < 0 = previous slot
        if event.y > 0.0 {
            inventory.active_slot = (inventory.active_slot + 1) % 9;
        } else if event.y < 0.0 {
            if inventory.active_slot == 0 {
                inventory.active_slot = 8;
            } else {
                inventory.active_slot -= 1;
            }
        }
    }

    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if keys.just_pressed(key) {
            inventory.active_slot = slot;
        }
    }
}

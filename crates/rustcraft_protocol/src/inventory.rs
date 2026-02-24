use crate::block::BlockType;

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

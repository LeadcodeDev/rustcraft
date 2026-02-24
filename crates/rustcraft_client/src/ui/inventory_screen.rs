use bevy::prelude::*;

use crate::ClientTransportRes;
use crate::events::{InventoryDroppedEvent, InventoryPickedUpEvent};
use rustcraft_protocol::protocol::ClientMessage;
use crate::inventory::{Inventory, ItemStack, MAX_STACK};
use crate::player::camera::{FlyCam, GameState, Player};
use crate::ui::block_preview::BlockPreviews;

#[derive(Component)]
pub struct InventoryScreenRoot;

#[derive(Component)]
pub struct InventorySlotButton(pub usize);

#[derive(Component)]
pub struct InventorySlotPreview(pub usize);

#[derive(Component)]
pub struct InventorySlotCount(pub usize);

/// Floating image that follows the cursor during drag.
#[derive(Component)]
pub struct DragGhost;

#[derive(Component)]
pub struct DragGhostCount;

/// Tracks the current drag state.
#[derive(Resource, Default)]
pub struct DragState {
    pub from_slot: Option<usize>,
    pub stack: Option<ItemStack>,
}

impl DragState {
    pub fn is_dragging(&self) -> bool {
        self.stack.is_some()
    }

    pub fn clear(&mut self) {
        self.from_slot = None;
        self.stack = None;
    }
}

const SLOT_SIZE: f32 = 44.0;
const SLOT_GAP: f32 = 4.0;
const PREVIEW_SIZE: f32 = 32.0;

pub fn spawn_inventory_screen(mut commands: Commands) {
    commands
        .spawn((
            InventoryScreenRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Inventory"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ));

            // Inventory grid: 3 rows of 9 (slots 9..35)
            for row in 0..3 {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(SLOT_GAP),
                        ..default()
                    })
                    .with_children(|row_node| {
                        for col in 0..9 {
                            let slot_index = 9 + row * 9 + col;
                            spawn_slot(row_node, slot_index);
                        }
                    });
            }

            // Separator
            parent.spawn((
                Node {
                    width: Val::Px(9.0 * SLOT_SIZE + 8.0 * SLOT_GAP),
                    height: Val::Px(2.0),
                    margin: UiRect::vertical(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
            ));

            // Hotbar row (slots 0..8)
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(SLOT_GAP),
                    ..default()
                })
                .with_children(|row_node| {
                    for col in 0..9 {
                        spawn_slot(row_node, col);
                    }
                });
        });

    // Drag ghost — floating preview that follows cursor
    commands
        .spawn((
            DragGhost,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(PREVIEW_SIZE),
                height: Val::Px(PREVIEW_SIZE),
                justify_content: JustifyContent::End,
                align_items: AlignItems::End,
                ..default()
            },
            ZIndex(100),
            Visibility::Hidden,
        ))
        .with_children(|ghost| {
            ghost.spawn((
                DragGhostCount,
                Text::new(""),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_slot(parent: &mut ChildBuilder, slot_index: usize) {
    parent
        .spawn((
            InventorySlotButton(slot_index),
            Button,
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
            BorderColor(Color::srgba(0.4, 0.4, 0.4, 0.8)),
        ))
        .with_children(|slot| {
            slot.spawn((
                InventorySlotPreview(slot_index),
                Node {
                    width: Val::Px(PREVIEW_SIZE),
                    height: Val::Px(PREVIEW_SIZE),
                    ..default()
                },
            ));
            slot.spawn((
                InventorySlotCount(slot_index),
                Text::new(""),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(1.0),
                    right: Val::Px(3.0),
                    ..default()
                },
            ));
        });
}

pub fn show_hide_inventory_screen(
    game_state: Res<GameState>,
    mut query: Query<(&mut Visibility, &mut Node), With<InventoryScreenRoot>>,
    mut drag_state: ResMut<DragState>,
    mut inventory: ResMut<Inventory>,
    mut ghost_query: Query<&mut Visibility, (With<DragGhost>, Without<InventoryScreenRoot>)>,
) {
    if !game_state.is_changed() {
        return;
    }
    let visible = *game_state == GameState::InInventory;
    for (mut vis, mut node) in &mut query {
        if visible {
            *vis = Visibility::Visible;
            node.display = Display::Flex;
        } else {
            *vis = Visibility::Hidden;
            node.display = Display::None;
        }
    }
    // Return dragged items to source slot when closing inventory
    if !visible && drag_state.is_dragging() {
        if let (Some(from_slot), Some(stack)) = (drag_state.from_slot, drag_state.stack) {
            // Try to return to source slot
            if let Some(existing) = &mut inventory.slots[from_slot] {
                if existing.block == stack.block {
                    existing.count = (existing.count + stack.count).min(MAX_STACK);
                }
            } else {
                inventory.slots[from_slot] = Some(stack);
            }
        }
        drag_state.clear();
        for mut vis in &mut ghost_query {
            *vis = Visibility::Hidden;
        }
    }
}

pub fn update_inventory_screen(
    game_state: Res<GameState>,
    inventory: Res<Inventory>,
    previews: Res<BlockPreviews>,
    mut preview_query: Query<(
        &InventorySlotPreview,
        Option<&mut ImageNode>,
        &mut Visibility,
        Entity,
    )>,
    mut count_query: Query<(&InventorySlotCount, &mut Text)>,
    mut commands: Commands,
) {
    if *game_state != GameState::InInventory {
        return;
    }

    for (slot_preview, image_node, mut vis, entity) in &mut preview_query {
        let idx = slot_preview.0;
        if let Some(stack) = inventory.slots[idx] {
            *vis = Visibility::Visible;
            if let Some(handle) = previews.get(stack.block) {
                if let Some(mut img) = image_node {
                    img.image = handle;
                } else {
                    commands.entity(entity).insert(ImageNode::new(handle));
                }
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }

    for (slot_count, mut text) in &mut count_query {
        let idx = slot_count.0;
        if let Some(stack) = inventory.slots[idx] {
            if stack.count > 1 {
                **text = stack.count.to_string();
            } else {
                **text = String::new();
            }
        } else {
            **text = String::new();
        }
    }
}

fn update_ghost(
    drag_state: &DragState,
    previews: &BlockPreviews,
    ghost_query: &mut Query<
        (&mut Visibility, &mut Node, Option<&mut ImageNode>, Entity),
        (With<DragGhost>, Without<DragGhostCount>),
    >,
    ghost_count_query: &mut Query<&mut Text, With<DragGhostCount>>,
    commands: &mut Commands,
) {
    if let Some(stack) = drag_state.stack {
        if let Some(handle) = previews.get(stack.block) {
            for (mut vis, _, image_node, entity) in ghost_query.iter_mut() {
                *vis = Visibility::Visible;
                if let Some(mut img) = image_node {
                    img.image = handle.clone();
                } else {
                    commands
                        .entity(entity)
                        .insert(ImageNode::new(handle.clone()));
                }
            }
        }
        for mut text in ghost_count_query.iter_mut() {
            if stack.count > 1 {
                **text = stack.count.to_string();
            } else {
                **text = String::new();
            }
        }
    } else {
        for (mut vis, _, _, _) in ghost_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for mut text in ghost_count_query.iter_mut() {
            **text = String::new();
        }
    }
}

pub fn drag_and_drop(
    game_state: Res<GameState>,
    mut drag_state: ResMut<DragState>,
    mut inventory: ResMut<Inventory>,
    previews: Res<BlockPreviews>,
    mouse: Res<ButtonInput<MouseButton>>,
    slot_query: Query<(&Interaction, &InventorySlotButton)>,
    mut ghost_query: Query<
        (&mut Visibility, &mut Node, Option<&mut ImageNode>, Entity),
        (With<DragGhost>, Without<DragGhostCount>),
    >,
    mut ghost_count_query: Query<&mut Text, With<DragGhostCount>>,
    windows: Query<&Window>,
    camera_query: Query<(&Transform, &Player), With<FlyCam>>,
    mut commands: Commands,
    mut ev_picked: EventWriter<InventoryPickedUpEvent>,
    mut ev_dropped: EventWriter<InventoryDroppedEvent>,
    transport: Res<ClientTransportRes>,
) {
    if *game_state != GameState::InInventory {
        return;
    }

    // Update ghost position to follow cursor
    if let Ok(window) = windows.get_single() {
        if let Some(cursor) = window.cursor_position() {
            for (_, mut node, _, _) in &mut ghost_query {
                node.left = Val::Px(cursor.x - PREVIEW_SIZE / 2.0);
                node.top = Val::Px(cursor.y - PREVIEW_SIZE / 2.0);
            }
        }
    }

    let left_pressed = mouse.just_pressed(MouseButton::Left);
    let right_pressed = mouse.just_pressed(MouseButton::Right);

    if !left_pressed && !right_pressed {
        return;
    }

    let Ok((transform, player)) = camera_query.get_single() else {
        return;
    };
    let location = player.location(transform);

    // Check if any slot is being clicked
    let mut clicked_slot: Option<usize> = None;
    for (interaction, slot_btn) in &slot_query {
        if *interaction == Interaction::Pressed {
            clicked_slot = Some(slot_btn.0);
            break;
        }
    }

    if let Some(slot_idx) = clicked_slot {
        if drag_state.is_dragging() {
            if left_pressed {
                // Drop drag onto slot
                let drag_stack = drag_state.stack.unwrap();

                if let Some(existing) = &mut inventory.slots[slot_idx] {
                    if existing.block == drag_stack.block {
                        // Merge same type
                        let space = MAX_STACK - existing.count;
                        let add = drag_stack.count.min(space);
                        existing.count += add;
                        let remaining = drag_stack.count - add;
                        if remaining > 0 {
                            drag_state.stack = Some(ItemStack::new(drag_stack.block, remaining));
                        } else {
                            ev_dropped.send(InventoryDroppedEvent {
                                from_slot: drag_state.from_slot.unwrap_or(0),
                                to_slot: slot_idx,
                                block_type: drag_stack.block,
                                count: drag_stack.count,
                                player: location,
                            });
                            drag_state.clear();
                        }
                    } else {
                        // Swap different types
                        let old = *existing;
                        inventory.slots[slot_idx] = Some(drag_stack);
                        ev_dropped.send(InventoryDroppedEvent {
                            from_slot: drag_state.from_slot.unwrap_or(0),
                            to_slot: slot_idx,
                            block_type: drag_stack.block,
                            count: drag_stack.count,
                            player: location,
                        });
                        drag_state.from_slot = Some(slot_idx);
                        drag_state.stack = Some(old);
                    }
                } else {
                    // Empty slot — place drag
                    inventory.slots[slot_idx] = Some(drag_stack);
                    ev_dropped.send(InventoryDroppedEvent {
                        from_slot: drag_state.from_slot.unwrap_or(0),
                        to_slot: slot_idx,
                        block_type: drag_stack.block,
                        count: drag_stack.count,
                        player: location,
                    });
                    drag_state.clear();
                }
            } else if right_pressed {
                // Right click while dragging on a slot with same type: pick one more
                if let Some(existing) = &mut inventory.slots[slot_idx] {
                    if let Some(drag_stack) = &mut drag_state.stack {
                        if existing.block == drag_stack.block && drag_stack.count < MAX_STACK {
                            drag_stack.count += 1;
                            existing.count -= 1;
                            if existing.count == 0 {
                                inventory.slots[slot_idx] = None;
                            }
                        }
                    }
                }
            }
        } else {
            // No drag active — pick up
            if left_pressed {
                // Left click: take entire stack
                if let Some(stack) = inventory.slots[slot_idx].take() {
                    ev_picked.send(InventoryPickedUpEvent {
                        slot: slot_idx,
                        block_type: stack.block,
                        count: stack.count,
                        player: location,
                    });
                    drag_state.from_slot = Some(slot_idx);
                    drag_state.stack = Some(stack);
                }
            } else if right_pressed {
                // Right click: take 1 item
                if let Some(existing) = &mut inventory.slots[slot_idx] {
                    let block = existing.block;
                    existing.count -= 1;
                    if existing.count == 0 {
                        inventory.slots[slot_idx] = None;
                    }
                    ev_picked.send(InventoryPickedUpEvent {
                        slot: slot_idx,
                        block_type: block,
                        count: 1,
                        player: location,
                    });
                    drag_state.from_slot = Some(slot_idx);
                    drag_state.stack = Some(ItemStack::new(block, 1));
                }
            }
        }
    } else if drag_state.is_dragging() && (left_pressed || right_pressed) {
        // Click outside inventory — drop to world via server
        let forward = transform.forward().as_vec3();
        let from_slot = drag_state.from_slot.unwrap_or(0);
        if left_pressed {
            // Left click: drop entire dragged stack
            let stack = drag_state.stack.unwrap();
            transport.0.send(ClientMessage::DropItem {
                slot: from_slot,
                count: stack.count,
                direction: forward,
            });
            drag_state.clear();
        } else {
            // Right click: drop 1 item
            let drag_stack = drag_state.stack.as_mut().unwrap();
            transport.0.send(ClientMessage::DropItem {
                slot: from_slot,
                count: 1,
                direction: forward,
            });
            drag_stack.count -= 1;
            if drag_stack.count == 0 {
                drag_state.clear();
            }
        }
    }

    update_ghost(
        &drag_state,
        &previews,
        &mut ghost_query,
        &mut ghost_count_query,
        &mut commands,
    );
}

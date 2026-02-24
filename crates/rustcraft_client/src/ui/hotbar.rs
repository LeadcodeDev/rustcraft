use bevy::prelude::*;

use crate::app_state::AppState;
use crate::inventory::Inventory;
use crate::player::camera::GameState;
use crate::ui::block_preview::BlockPreviews;

#[derive(Component)]
pub struct HotbarRoot;

#[derive(Component)]
pub struct HotbarSlot(pub usize);

#[derive(Component)]
pub struct HotbarSlotPreview(pub usize);

#[derive(Component)]
pub struct HotbarSlotCount(pub usize);

const SLOT_SIZE: f32 = 44.0;
const SLOT_GAP: f32 = 4.0;
const PREVIEW_SIZE: f32 = 32.0;

pub fn spawn_hotbar(mut commands: Commands) {
    commands
        .spawn((
            HotbarRoot,
            StateScoped(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(SLOT_GAP),
                ..default()
            },
            Visibility::Visible,
        ))
        .with_children(|parent| {
            for i in 0..9 {
                parent
                    .spawn((
                        HotbarSlot(i),
                        Node {
                            width: Val::Px(SLOT_SIZE),
                            height: Val::Px(SLOT_SIZE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 0.8)),
                        BorderColor(Color::srgba(0.4, 0.4, 0.4, 0.8)),
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            HotbarSlotPreview(i),
                            Node {
                                width: Val::Px(PREVIEW_SIZE),
                                height: Val::Px(PREVIEW_SIZE),
                                ..default()
                            },
                        ));
                        slot.spawn((
                            HotbarSlotCount(i),
                            Text::new(""),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(2.0),
                                right: Val::Px(4.0),
                                ..default()
                            },
                        ));
                    });
            }
        });
}

pub fn show_hide_hotbar(
    game_state: Res<GameState>,
    mut query: Query<(&mut Visibility, &mut Node), With<HotbarRoot>>,
) {
    if !game_state.is_changed() {
        return;
    }
    for (mut vis, mut node) in &mut query {
        if *game_state == GameState::InInventory {
            *vis = Visibility::Hidden;
            node.display = Display::None;
        } else {
            *vis = Visibility::Visible;
            node.display = Display::Flex;
        }
    }
}

pub fn update_hotbar(
    game_state: Res<GameState>,
    inventory: Res<Inventory>,
    previews: Res<BlockPreviews>,
    mut slot_query: Query<(&HotbarSlot, &mut BorderColor)>,
    mut preview_query: Query<(
        &HotbarSlotPreview,
        Option<&mut ImageNode>,
        &mut Visibility,
        Entity,
    )>,
    mut count_query: Query<(&HotbarSlotCount, &mut Text)>,
    mut commands: Commands,
) {
    if *game_state == GameState::InInventory {
        return;
    }

    for (slot, mut border) in &mut slot_query {
        if slot.0 == inventory.active_slot {
            *border = BorderColor(Color::WHITE);
        } else {
            *border = BorderColor(Color::srgba(0.4, 0.4, 0.4, 0.8));
        }
    }

    for (preview, image_node, mut vis, entity) in &mut preview_query {
        if let Some(stack) = inventory.slots[preview.0] {
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

    for (count_slot, mut text) in &mut count_query {
        if let Some(stack) = inventory.slots[count_slot.0] {
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

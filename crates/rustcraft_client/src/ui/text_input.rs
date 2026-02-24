use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

/// A minimal text input field component.
#[derive(Component)]
pub struct TextInput {
    pub value: String,
    pub placeholder: String,
    pub focused: bool,
    pub max_length: usize,
}

/// Marker for the Text entity displaying the input value.
#[derive(Component)]
pub struct TextInputDisplay;

/// Links a display text to its parent TextInput entity.
#[derive(Component)]
pub struct TextInputOf(pub Entity);

pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (text_input_focus, text_input_keyboard, text_input_render),
        );
    }
}

/// Spawn a text input field with an optional extra marker bundle. Returns the root entity.
pub fn spawn_text_input(
    parent: &mut ChildBuilder,
    placeholder: &str,
    max_length: usize,
    width: f32,
    extra: impl Bundle,
) -> Entity {
    let placeholder_owned = placeholder.to_string();
    let mut entity_cmd = parent.spawn((
        extra,
        TextInput {
            value: String::new(),
            placeholder: placeholder_owned.clone(),
            focused: false,
            max_length,
        },
        Button,
        Node {
            width: Val::Px(width),
            height: Val::Px(40.0),
            padding: UiRect::horizontal(Val::Px(10.0)),
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor(Color::srgb(0.4, 0.4, 0.4)),
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
    ));
    let input_entity = entity_cmd.id();
    entity_cmd.with_children(|input| {
        input.spawn((
            TextInputDisplay,
            TextInputOf(input_entity),
            Text::new(placeholder_owned),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        ));
    });

    input_entity
}

/// Focus/unfocus text inputs on click.
fn text_input_focus(
    mut query: Query<(Entity, &Interaction, &mut TextInput, &mut BorderColor), Changed<Interaction>>,
) {
    // We need to handle this differently since we can't query the same component mutably twice.
    // Collect which entity was clicked.
    let mut clicked_entity = None;

    for (entity, interaction, _, _) in &query {
        if *interaction == Interaction::Pressed {
            clicked_entity = Some(entity);
        }
    }

    if let Some(clicked) = clicked_entity {
        // Unfocus all, then focus the clicked one
        for (entity, _, mut input, mut border) in &mut query {
            if entity == clicked {
                input.focused = true;
                *border = BorderColor(Color::srgb(0.3, 0.6, 1.0));
            } else {
                input.focused = false;
                *border = BorderColor(Color::srgb(0.4, 0.4, 0.4));
            }
        }
    }
}

/// Handle keyboard input for focused text inputs.
fn text_input_keyboard(
    mut events: EventReader<KeyboardInput>,
    mut query: Query<&mut TextInput>,
) {
    for event in events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        for mut input in &mut query {
            if !input.focused {
                continue;
            }

            match &event.logical_key {
                Key::Character(c) => {
                    if input.value.len() < input.max_length {
                        input.value.push_str(c.as_str());
                    }
                }
                Key::Backspace => {
                    input.value.pop();
                }
                _ => {}
            }
        }
    }
}

/// Update the display text to match the input value.
fn text_input_render(
    inputs: Query<&TextInput, Changed<TextInput>>,
    mut displays: Query<(&TextInputOf, &mut Text, &mut TextColor), With<TextInputDisplay>>,
) {
    for (of, mut text, mut color) in &mut displays {
        let Ok(input) = inputs.get(of.0) else {
            continue;
        };

        if input.value.is_empty() {
            **text = input.placeholder.clone();
            *color = TextColor(Color::srgb(0.5, 0.5, 0.5));
        } else {
            **text = input.value.clone();
            *color = TextColor(Color::WHITE);
        }
    }
}

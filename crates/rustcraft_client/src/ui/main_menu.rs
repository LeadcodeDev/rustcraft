use std::path::Path;

use bevy::prelude::*;

use crate::app_state::{AppState, ConnectionConfig};
use super::text_input::{TextInput, spawn_text_input};

// --- Sub-state for menu screens ---

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuScreen {
    #[default]
    Root,
    MultiJoin,
}

// --- Component markers ---

#[derive(Component)]
struct RootMenuScreen;

#[derive(Component)]
struct MultiJoinScreen;

#[derive(Component)]
struct SoloButton;

#[derive(Component)]
struct MultiButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct JoinButton;

#[derive(Component)]
struct BackButton;

#[derive(Component)]
struct PlayerNameInput;

#[derive(Component)]
struct AddressInput;

#[derive(Component)]
struct AuthCodeInput;

// --- Plugin ---

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_menu_camera)
            .add_systems(OnEnter(MenuScreen::Root), spawn_root_menu)
            .add_systems(OnExit(MenuScreen::Root), cleanup::<RootMenuScreen>)
            .add_systems(OnEnter(MenuScreen::MultiJoin), spawn_multi_join)
            .add_systems(OnExit(MenuScreen::MultiJoin), cleanup::<MultiJoinScreen>)
            .add_systems(
                Update,
                (
                    handle_root_buttons.run_if(in_state(MenuScreen::Root)),
                    handle_multi_join_buttons.run_if(in_state(MenuScreen::MultiJoin)),
                    menu_button_hover.run_if(in_state(AppState::MainMenu)),
                ),
            );
    }
}

fn spawn_menu_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        StateScoped(AppState::MainMenu),
    ));
}

// --- Shared UI helpers ---

const BUTTON_WIDTH: f32 = 300.0;
const BUTTON_HEIGHT: f32 = 60.0;
const BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
const BUTTON_HOVER: Color = Color::srgb(0.4, 0.4, 0.4);
const DANGER_COLOR: Color = Color::srgb(0.5, 0.15, 0.15);
const DANGER_HOVER: Color = Color::srgb(0.6, 0.2, 0.2);

#[derive(Component)]
struct MenuButton {
    base_color: Color,
    hover_color: Color,
}

fn spawn_menu_button(
    parent: &mut ChildBuilder,
    label: &str,
    marker: impl Bundle,
    danger: bool,
) {
    let (base, hover) = if danger {
        (DANGER_COLOR, DANGER_HOVER)
    } else {
        (BUTTON_COLOR, BUTTON_HOVER)
    };

    parent
        .spawn((
            marker,
            MenuButton {
                base_color: base,
                hover_color: hover,
            },
            Button,
            Node {
                width: Val::Px(BUTTON_WIDTH),
                height: Val::Px(BUTTON_HEIGHT),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(base),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn menu_button_hover(
    mut query: Query<
        (&Interaction, &MenuButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, button, mut bg) in &mut query {
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => {
                BackgroundColor(button.hover_color)
            }
            Interaction::None => BackgroundColor(button.base_color),
        };
    }
}

fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

// --- Root menu screen ---

fn spawn_root_menu(mut commands: Commands) {
    commands
        .spawn((
            RootMenuScreen,
            StateScoped(AppState::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("RUSTCRAFT"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            spawn_menu_button(parent, "Jouer en solo", SoloButton, false);
            spawn_menu_button(parent, "Jouer en multi", MultiButton, false);
            spawn_menu_button(parent, "Quitter", QuitButton, true);
        });
}

fn handle_root_buttons(
    solo_query: Query<&Interaction, (Changed<Interaction>, With<SoloButton>)>,
    multi_query: Query<&Interaction, (Changed<Interaction>, With<MultiButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_menu_screen: ResMut<NextState<MenuScreen>>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Solo: immediate launch
    for &interaction in &solo_query {
        if interaction == Interaction::Pressed {
            let world_exists = Path::new("worlds/default/world.dat").exists();
            let seed = if world_exists {
                0 // Will be overridden by load_or_create from disk
            } else {
                rand::random::<u32>()
            };

            commands.insert_resource(ConnectionConfig::Solo {
                world_name: "default".to_string(),
                seed,
                player_name: "Player".to_string(),
            });
            next_app_state.set(AppState::InGame);
        }
    }

    // Multi: go to join screen
    for &interaction in &multi_query {
        if interaction == Interaction::Pressed {
            next_menu_screen.set(MenuScreen::MultiJoin);
        }
    }

    // Quit
    for &interaction in &quit_query {
        if interaction == Interaction::Pressed {
            app_exit.send(AppExit::Success);
        }
    }
}

// --- Multi join screen ---

fn spawn_multi_join(mut commands: Commands) {
    commands
        .spawn((
            MultiJoinScreen,
            StateScoped(AppState::MainMenu),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Jouer en multi"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            // Player name field
            spawn_labeled_input(parent, "Nom du joueur", "Player", 32, PlayerNameInput);

            // Address field
            spawn_labeled_input(parent, "Adresse (host:port)", "127.0.0.1:25565", 64, AddressInput);

            // Auth code field
            spawn_labeled_input(parent, "Code d'authentification", "ABC123", 6, AuthCodeInput);

            // Button row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    margin: UiRect::top(Val::Px(24.0)),
                    ..default()
                })
                .with_children(|row| {
                    spawn_menu_button(row, "Rejoindre", JoinButton, false);
                    spawn_menu_button(row, "Retour", BackButton, true);
                });
        });
}

fn spawn_labeled_input(
    parent: &mut ChildBuilder,
    label: &str,
    placeholder: &str,
    max_length: usize,
    marker: impl Bundle,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                Node {
                    width: Val::Px(220.0),
                    ..default()
                },
            ));

            spawn_text_input(row, placeholder, max_length, 250.0, marker);
        });
}

fn handle_multi_join_buttons(
    join_query: Query<&Interaction, (Changed<Interaction>, With<JoinButton>)>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    player_name_query: Query<&TextInput, With<PlayerNameInput>>,
    address_query: Query<&TextInput, With<AddressInput>>,
    auth_code_query: Query<&TextInput, With<AuthCodeInput>>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_menu_screen: ResMut<NextState<MenuScreen>>,
) {
    // Join
    for &interaction in &join_query {
        if interaction == Interaction::Pressed {
            let player_name = player_name_query
                .iter()
                .next()
                .map(|i| i.value.clone())
                .unwrap_or_else(|| "Player".to_string());
            let address = address_query
                .iter()
                .next()
                .map(|i| i.value.clone())
                .unwrap_or_default();
            let auth_code = auth_code_query
                .iter()
                .next()
                .map(|i| i.value.clone())
                .unwrap_or_default();

            if address.is_empty() || auth_code.is_empty() {
                return;
            }

            let player_name = if player_name.is_empty() {
                "Player".to_string()
            } else {
                player_name
            };

            commands.insert_resource(ConnectionConfig::Multi {
                address,
                auth_code,
                player_name,
            });
            next_app_state.set(AppState::InGame);
        }
    }

    // Back
    for &interaction in &back_query {
        if interaction == Interaction::Pressed {
            next_menu_screen.set(MenuScreen::Root);
        }
    }
}

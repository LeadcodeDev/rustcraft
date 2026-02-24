use bevy::prelude::*;

use crate::player::camera::GameState;

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub struct ResumeButton;

#[derive(Component)]
pub struct QuitButton;

pub fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenuRoot,
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Paused"),
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

            // Resume button
            parent
                .spawn((
                    ResumeButton,
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Resume"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Quit button
            parent
                .spawn((
                    QuitButton,
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.15, 0.15)),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Quit"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn show_hide_pause_menu(
    game_state: Res<GameState>,
    mut query: Query<&mut Visibility, With<PauseMenuRoot>>,
) {
    if !game_state.is_changed() {
        return;
    }
    for mut vis in &mut query {
        *vis = match *game_state {
            GameState::Paused => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}

pub fn handle_resume_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window>,
) {
    for &inter in &interaction {
        if inter == Interaction::Pressed {
            *game_state = GameState::Playing;
            if let Ok(mut window) = windows.get_single_mut() {
                window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

pub fn handle_quit_button(
    interaction: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    mut app_exit: EventWriter<AppExit>,
) {
    for &inter in &interaction {
        if inter == Interaction::Pressed {
            app_exit.send(AppExit::Success);
        }
    }
}

pub fn button_hover(
    mut query: Query<
        (&Interaction, &mut BackgroundColor, Option<&ResumeButton>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg, is_resume) in &mut query {
        let base = if is_resume.is_some() {
            Color::srgb(0.3, 0.3, 0.3)
        } else {
            Color::srgb(0.5, 0.15, 0.15)
        };
        let hover = if is_resume.is_some() {
            Color::srgb(0.4, 0.4, 0.4)
        } else {
            Color::srgb(0.6, 0.2, 0.2)
        };

        *bg = match interaction {
            Interaction::Hovered => BackgroundColor(hover),
            _ => BackgroundColor(base),
        };
    }
}

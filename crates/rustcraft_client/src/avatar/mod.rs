use std::f32::consts::PI;

use bevy::prelude::*;

use crate::events::{PlayerJoinEvent, PlayerLeaveEvent};
use crate::inventory::Inventory;
use crate::network::RemotePlayerStates;
use crate::player::camera::{FlyCam, GameState, Player, EYE_HEIGHT};
use crate::world::block::{BlockColor, BlockType};

// --- Colors ---

const SKIN_COLOR: Color = Color::srgb(0.87, 0.72, 0.58);
const SHIRT_COLOR: Color = Color::srgb(0.30, 0.55, 0.78);
const PANTS_COLOR: Color = Color::srgb(0.35, 0.30, 0.25);
const SHOE_COLOR: Color = Color::srgb(0.25, 0.20, 0.15);

// --- Animation ---

const WALK_SWING_SPEED: f32 = 10.0;
const ARM_SWING_ANGLE: f32 = 0.6;
const LEG_SWING_ANGLE: f32 = 0.5;
const LOWER_BEND_ANGLE: f32 = 0.3;
const SWING_LERP_SPEED: f32 = 8.0;

// --- Third person ---

const THIRD_PERSON_DISTANCE: f32 = 4.0;

// --- Components ---

#[derive(Component)]
pub struct PlayerAvatar;

#[derive(Component)]
pub struct FirstPersonHands;

#[derive(Component)]
pub struct FirstPersonArm {
    pub side: f32, // -1.0 = left, 1.0 = right
    pub base_translation: Vec3,
    pub base_rotation: Quat,
}

#[derive(Component)]
pub struct HeldBlockDisplay {
    pub current_block: Option<BlockType>,
}

#[derive(Component)]
pub struct FpHeldBlock;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BodyPart {
    Head,
    Torso,
    LeftUpperArm,
    LeftLowerArm,
    RightUpperArm,
    RightLowerArm,
    LeftUpperLeg,
    LeftLowerLeg,
    RightUpperLeg,
    RightLowerLeg,
}

#[derive(Component)]
pub struct AvatarAnimation {
    pub walk_phase: f32,
    pub swing_amplitude: f32,
    pub last_position: Vec3,
}

#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
}

// --- Remote player components ---

#[derive(Component)]
pub struct RemotePlayer {
    pub id: u64,
}

#[derive(Component)]
pub struct RemotePlayerNameTag;

// --- Plugin ---

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .add_systems(
                OnEnter(crate::app_state::AppState::InGame),
                spawn_avatar.after(crate::player::camera::spawn_camera),
            )
            .add_systems(
                Update,
                (
                    toggle_camera_mode,
                    sync_avatar_position
                        .after(crate::player::camera::camera_movement),
                    animate_avatar_walk.after(sync_avatar_position),
                    animate_first_person_hands.after(animate_avatar_walk),
                    update_held_block,
                    adjust_camera_for_mode
                        .after(crate::player::camera::camera_movement)
                        .after(sync_avatar_position),
                    spawn_remote_player,
                    despawn_remote_player,
                    update_remote_players,
                )
                    .run_if(in_state(crate::app_state::AppState::InGame)),
            );
    }
}

// --- Spawn ---

fn spawn_pivot() -> (
    Transform,
    GlobalTransform,
    Visibility,
    InheritedVisibility,
    ViewVisibility,
) {
    (
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Inherited,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )
}

fn spawn_avatar(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<(Entity, &Player), With<FlyCam>>,
) {
    let Ok((camera_entity, player)) = player_query.get_single() else {
        return;
    };

    let skin_mat = materials.add(StandardMaterial {
        base_color: SKIN_COLOR,
        ..default()
    });
    let shirt_mat = materials.add(StandardMaterial {
        base_color: SHIRT_COLOR,
        ..default()
    });
    let pants_mat = materials.add(StandardMaterial {
        base_color: PANTS_COLOR,
        ..default()
    });
    let shoe_mat = materials.add(StandardMaterial {
        base_color: SHOE_COLOR,
        ..default()
    });

    let held_block_mesh = meshes.add(Cuboid::new(0.14, 0.14, 0.14));
    let held_block_mat_3p = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    let held_block_mat_fp = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

    let head_mesh = meshes.add(Cuboid::new(0.50, 0.50, 0.50));
    let torso_mesh = meshes.add(Cuboid::new(0.60, 0.55, 0.30));
    let upper_arm_mesh = meshes.add(Cuboid::new(0.20, 0.30, 0.20));
    let lower_arm_mesh = meshes.add(Cuboid::new(0.18, 0.28, 0.18));
    let upper_leg_mesh = meshes.add(Cuboid::new(0.25, 0.35, 0.25));
    let lower_leg_mesh = meshes.add(Cuboid::new(0.22, 0.35, 0.22));

    commands
        .spawn((
            PlayerAvatar,
            StateScoped(crate::app_state::AppState::InGame),
            AvatarAnimation {
                walk_phase: 0.0,
                swing_amplitude: 0.0,
                last_position: player.position,
            },
            Transform::from_translation(player.position),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .with_children(|root| {
            // Torso
            root.spawn((
                BodyPart::Torso,
                Mesh3d(torso_mesh),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.975, 0.0)),
            ));

            // Head pivot
            let mut head_pivot = spawn_pivot();
            head_pivot.0.translation = Vec3::new(0.0, 1.30, 0.0);
            root.spawn((BodyPart::Head, head_pivot.0, head_pivot.1, head_pivot.2, head_pivot.3, head_pivot.4))
                .with_children(|head| {
                    head.spawn((
                        Mesh3d(head_mesh),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_translation(Vec3::new(0.0, 0.25, 0.0)),
                    ));
                });

            // Left Arm
            let mut la_pivot = spawn_pivot();
            la_pivot.0.translation = Vec3::new(0.40, 1.25, 0.0);
            root.spawn((BodyPart::LeftUpperArm, la_pivot.0, la_pivot.1, la_pivot.2, la_pivot.3, la_pivot.4))
                .with_children(|shoulder| {
                    shoulder.spawn((
                        Mesh3d(upper_arm_mesh.clone()),
                        MeshMaterial3d(shirt_mat.clone()),
                        Transform::from_translation(Vec3::new(0.0, -0.15, 0.0)),
                    ));
                    let mut elbow = spawn_pivot();
                    elbow.0.translation = Vec3::new(0.0, -0.30, 0.0);
                    shoulder
                        .spawn((BodyPart::LeftLowerArm, elbow.0, elbow.1, elbow.2, elbow.3, elbow.4))
                        .with_children(|e| {
                            e.spawn((
                                Mesh3d(lower_arm_mesh.clone()),
                                MeshMaterial3d(skin_mat.clone()),
                                Transform::from_translation(Vec3::new(0.0, -0.14, 0.0)),
                            ));
                            // Held block (3rd person) — right hand on screen (left arm of avatar)
                            e.spawn((
                                HeldBlockDisplay { current_block: None },
                                Mesh3d(held_block_mesh.clone()),
                                MeshMaterial3d(held_block_mat_3p),
                                Transform::from_translation(Vec3::new(0.0, -0.38, 0.0)),
                                Visibility::Hidden,
                            ));
                        });
                });

            // Right Arm
            let mut ra_pivot = spawn_pivot();
            ra_pivot.0.translation = Vec3::new(-0.40, 1.25, 0.0);
            root.spawn((BodyPart::RightUpperArm, ra_pivot.0, ra_pivot.1, ra_pivot.2, ra_pivot.3, ra_pivot.4))
                .with_children(|shoulder| {
                    shoulder.spawn((
                        Mesh3d(upper_arm_mesh.clone()),
                        MeshMaterial3d(shirt_mat.clone()),
                        Transform::from_translation(Vec3::new(0.0, -0.15, 0.0)),
                    ));
                    let mut elbow = spawn_pivot();
                    elbow.0.translation = Vec3::new(0.0, -0.30, 0.0);
                    shoulder
                        .spawn((BodyPart::RightLowerArm, elbow.0, elbow.1, elbow.2, elbow.3, elbow.4))
                        .with_children(|e| {
                            e.spawn((
                                Mesh3d(lower_arm_mesh.clone()),
                                MeshMaterial3d(skin_mat.clone()),
                                Transform::from_translation(Vec3::new(0.0, -0.14, 0.0)),
                            ));
                        });
                });

            // Left Leg
            let mut ll_pivot = spawn_pivot();
            ll_pivot.0.translation = Vec3::new(0.15, 0.70, 0.0);
            root.spawn((BodyPart::LeftUpperLeg, ll_pivot.0, ll_pivot.1, ll_pivot.2, ll_pivot.3, ll_pivot.4))
                .with_children(|hip| {
                    hip.spawn((
                        Mesh3d(upper_leg_mesh.clone()),
                        MeshMaterial3d(pants_mat.clone()),
                        Transform::from_translation(Vec3::new(0.0, -0.175, 0.0)),
                    ));
                    let mut knee = spawn_pivot();
                    knee.0.translation = Vec3::new(0.0, -0.35, 0.0);
                    hip.spawn((BodyPart::LeftLowerLeg, knee.0, knee.1, knee.2, knee.3, knee.4))
                        .with_children(|k| {
                            k.spawn((
                                Mesh3d(lower_leg_mesh.clone()),
                                MeshMaterial3d(shoe_mat.clone()),
                                Transform::from_translation(Vec3::new(0.0, -0.175, 0.0)),
                            ));
                        });
                });

            // Right Leg
            let mut rl_pivot = spawn_pivot();
            rl_pivot.0.translation = Vec3::new(-0.15, 0.70, 0.0);
            root.spawn((BodyPart::RightUpperLeg, rl_pivot.0, rl_pivot.1, rl_pivot.2, rl_pivot.3, rl_pivot.4))
                .with_children(|hip| {
                    hip.spawn((
                        Mesh3d(upper_leg_mesh),
                        MeshMaterial3d(pants_mat),
                        Transform::from_translation(Vec3::new(0.0, -0.175, 0.0)),
                    ));
                    let mut knee = spawn_pivot();
                    knee.0.translation = Vec3::new(0.0, -0.35, 0.0);
                    hip.spawn((BodyPart::RightLowerLeg, knee.0, knee.1, knee.2, knee.3, knee.4))
                        .with_children(|k| {
                            k.spawn((
                                Mesh3d(lower_leg_mesh),
                                MeshMaterial3d(shoe_mat),
                                Transform::from_translation(Vec3::new(0.0, -0.175, 0.0)),
                            ));
                        });
                });
        });

    // --- First-person hands (children of camera) ---
    let fp_arm_mesh = meshes.add(Cuboid::new(0.18, 0.50, 0.18));

    commands.entity(camera_entity).with_children(|cam| {
        cam.spawn((
            FirstPersonHands,
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .with_children(|hands| {
            let right_pos = Vec3::new(0.36, -0.35, -0.45);
            let right_rot = Quat::from_euler(EulerRot::XYZ, -1.4, 0.15, 0.0);
            hands.spawn((
                FirstPersonArm {
                    side: 1.0,
                    base_translation: right_pos,
                    base_rotation: right_rot,
                },
                Mesh3d(fp_arm_mesh.clone()),
                MeshMaterial3d(skin_mat.clone()),
                Transform {
                    translation: right_pos,
                    rotation: right_rot,
                    ..default()
                },
            ));

            // Held block (1st person) — direct child of hands, above right hand
            hands.spawn((
                FpHeldBlock,
                HeldBlockDisplay { current_block: None },
                Mesh3d(held_block_mesh),
                MeshMaterial3d(held_block_mat_fp),
                Transform::from_translation(Vec3::new(0.36, -0.20, -0.55)),
                Visibility::Hidden,
            ));

            let left_pos = Vec3::new(-0.36, -0.35, -0.45);
            let left_rot = Quat::from_euler(EulerRot::XYZ, -1.4, -0.15, 0.0);
            hands.spawn((
                FirstPersonArm {
                    side: -1.0,
                    base_translation: left_pos,
                    base_rotation: left_rot,
                },
                Mesh3d(fp_arm_mesh),
                MeshMaterial3d(skin_mat),
                Transform {
                    translation: left_pos,
                    rotation: left_rot,
                    ..default()
                },
            ));
        });
    });
}

// --- Sync systems ---

fn sync_avatar_position(
    player_query: Query<(&Player, &Transform), With<FlyCam>>,
    mut avatar_query: Query<&mut Transform, (With<PlayerAvatar>, Without<FlyCam>)>,
) {
    let Ok((player, cam_transform)) = player_query.get_single() else {
        return;
    };
    let Ok(mut avatar_transform) = avatar_query.get_single_mut() else {
        return;
    };

    avatar_transform.translation = player.position;

    let (yaw, _pitch, _roll) = cam_transform.rotation.to_euler(EulerRot::YXZ);
    avatar_transform.rotation = Quat::from_rotation_y(yaw);
}

// --- Animation ---

fn animate_avatar_walk(
    time: Res<Time>,
    player_query: Query<(&Player, &Transform), With<FlyCam>>,
    mut avatar_query: Query<&mut AvatarAnimation, With<PlayerAvatar>>,
    mut parts_query: Query<
        (&BodyPart, &mut Transform),
        (Without<PlayerAvatar>, Without<FlyCam>),
    >,
) {
    let Ok((player, cam_transform)) = player_query.get_single() else {
        return;
    };
    let Ok(mut anim) = avatar_query.get_single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // Detect walking via XZ delta
    let horizontal_delta = Vec2::new(
        player.position.x - anim.last_position.x,
        player.position.z - anim.last_position.z,
    );
    let is_walking = horizontal_delta.length() > 0.001;
    anim.last_position = player.position;

    // Lerp swing amplitude
    let target_amplitude = if is_walking { 1.0 } else { 0.0 };
    anim.swing_amplitude += (target_amplitude - anim.swing_amplitude) * SWING_LERP_SPEED * dt;

    if is_walking {
        anim.walk_phase += WALK_SWING_SPEED * dt;
    }

    let phase = anim.walk_phase;
    let amp = anim.swing_amplitude;

    // Head pitch from camera
    let (_yaw, pitch, _roll) = cam_transform.rotation.to_euler(EulerRot::YXZ);

    // Compute swing angles
    let left_arm_angle = phase.sin() * ARM_SWING_ANGLE * amp;
    let right_arm_angle = (phase + PI).sin() * ARM_SWING_ANGLE * amp;
    let left_leg_angle = (phase + PI).sin() * LEG_SWING_ANGLE * amp;
    let right_leg_angle = phase.sin() * LEG_SWING_ANGLE * amp;

    let left_lower_leg_bend = (left_leg_angle.max(0.0) / LEG_SWING_ANGLE.max(0.001)) * LOWER_BEND_ANGLE * amp;
    let right_lower_leg_bend = (right_leg_angle.max(0.0) / LEG_SWING_ANGLE.max(0.001)) * LOWER_BEND_ANGLE * amp;

    for (part, mut transform) in &mut parts_query {
        match part {
            BodyPart::Head => {
                transform.rotation = Quat::from_rotation_x(pitch);
            }
            BodyPart::LeftUpperArm => {
                transform.rotation = Quat::from_rotation_x(left_arm_angle);
            }
            BodyPart::RightUpperArm => {
                transform.rotation = Quat::from_rotation_x(right_arm_angle);
            }
            BodyPart::LeftUpperLeg => {
                transform.rotation = Quat::from_rotation_x(left_leg_angle);
            }
            BodyPart::RightUpperLeg => {
                transform.rotation = Quat::from_rotation_x(right_leg_angle);
            }
            BodyPart::LeftLowerLeg => {
                transform.rotation = Quat::from_rotation_x(left_lower_leg_bend);
            }
            BodyPart::RightLowerLeg => {
                transform.rotation = Quat::from_rotation_x(right_lower_leg_bend);
            }
            _ => {}
        }
    }
}

// --- First-person arm animation ---

fn animate_first_person_hands(
    camera_mode: Res<CameraMode>,
    avatar_query: Query<&AvatarAnimation, With<PlayerAvatar>>,
    mut arms_query: Query<(&FirstPersonArm, &mut Transform), Without<FpHeldBlock>>,
    mut held_block_query: Query<&mut Transform, With<FpHeldBlock>>,
) {
    if *camera_mode != CameraMode::FirstPerson {
        return;
    }

    let Ok(anim) = avatar_query.get_single() else {
        return;
    };

    let phase = anim.walk_phase;
    let amp = anim.swing_amplitude;

    // Base position of the FP held block (matches right hand)
    let block_base = Vec3::new(0.36, -0.20, -0.55);
    let mut right_swing = 0.0;
    let mut right_bob = 0.0;

    for (arm, mut transform) in &mut arms_query {
        // Opposite arms swing: right arm goes forward when left leg does (side=1.0 uses +PI offset)
        let swing = (phase + if arm.side > 0.0 { PI } else { 0.0 }).sin() * amp;

        // Vertical bob synced with walk
        let bob_y = (phase * 2.0).sin().abs() * 0.02 * amp;

        transform.translation = arm.base_translation + Vec3::new(0.0, bob_y, swing * 0.08);
        transform.rotation = arm.base_rotation * Quat::from_rotation_x(swing * 0.3);

        // Capture right arm motion for the held block
        if arm.side > 0.0 {
            right_swing = swing;
            right_bob = bob_y;
        }
    }

    // Move held block in sync with right arm
    if let Ok(mut block_transform) = held_block_query.get_single_mut() {
        block_transform.translation =
            block_base + Vec3::new(0.0, right_bob, right_swing * 0.08);
    }
}

// --- Held block update ---

fn update_held_block(
    inventory: Res<Inventory>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(
        &mut HeldBlockDisplay,
        &MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
) {
    let active = inventory.active_block();

    for (mut display, mat_handle, mut vis) in &mut query {
        if display.current_block == active {
            continue;
        }
        display.current_block = active;

        match active {
            Some(block) => {
                *vis = Visibility::Inherited;
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.base_color = block.color();
                }
            }
            None => {
                *vis = Visibility::Hidden;
            }
        }
    }
}

// --- Camera mode toggle ---

fn toggle_camera_mode(
    game_state: Res<GameState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_mode: ResMut<CameraMode>,
    mut hands_query: Query<&mut Visibility, With<FirstPersonHands>>,
) {
    if *game_state != GameState::Playing || !keys.just_pressed(KeyCode::F5) {
        return;
    }

    *camera_mode = match *camera_mode {
        CameraMode::FirstPerson => CameraMode::ThirdPerson,
        CameraMode::ThirdPerson => CameraMode::FirstPerson,
    };

    let third_person = *camera_mode == CameraMode::ThirdPerson;

    for mut vis in &mut hands_query {
        *vis = if third_person {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }

    info!("Camera: {:?}", if third_person { "ThirdPerson" } else { "FirstPerson" });
}

fn adjust_camera_for_mode(
    camera_mode: Res<CameraMode>,
    mut player_query: Query<(&Player, &mut Transform), With<FlyCam>>,
) {
    let Ok((player, mut cam_transform)) = player_query.get_single_mut() else {
        return;
    };

    let (yaw, pitch, _) = cam_transform.rotation.to_euler(EulerRot::YXZ);

    match *camera_mode {
        CameraMode::FirstPerson => {
            // In first person, camera_movement already positions the camera
            // with the forward offset. Nothing to do here.
        }
        CameraMode::ThirdPerson => {
            // Spherical offset: camera orbits behind the player
            let offset = Vec3::new(
                yaw.sin() * pitch.cos(),
                -pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * THIRD_PERSON_DISTANCE;

            let eye_center = player.position + Vec3::Y * EYE_HEIGHT;
            cam_transform.translation = eye_center + offset;
        }
    }
}

// --- Remote player systems ---

const REMOTE_LERP_SPEED: f32 = 12.0;

fn spawn_remote_player(
    mut commands: Commands,
    mut ev_join: EventReader<PlayerJoinEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in ev_join.read() {
        let skin_mat = materials.add(StandardMaterial {
            base_color: SKIN_COLOR,
            ..default()
        });
        let shirt_mat = materials.add(StandardMaterial {
            base_color: SHIRT_COLOR,
            ..default()
        });
        let pants_mat = materials.add(StandardMaterial {
            base_color: PANTS_COLOR,
            ..default()
        });
        let shoe_mat = materials.add(StandardMaterial {
            base_color: SHOE_COLOR,
            ..default()
        });

        let head_mesh = meshes.add(Cuboid::new(0.50, 0.50, 0.50));
        let torso_mesh = meshes.add(Cuboid::new(0.60, 0.55, 0.30));
        let arm_mesh = meshes.add(Cuboid::new(0.20, 0.55, 0.20));
        let leg_mesh = meshes.add(Cuboid::new(0.25, 0.65, 0.25));

        commands
            .spawn((
                RemotePlayer { id: event.player_id },
                Transform::from_translation(event.position),
                GlobalTransform::default(),
                Visibility::Visible,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .with_children(|root| {
                // Head
                root.spawn((
                    Mesh3d(head_mesh),
                    MeshMaterial3d(skin_mat),
                    Transform::from_translation(Vec3::new(0.0, 1.55, 0.0)),
                ));

                // Torso
                root.spawn((
                    Mesh3d(torso_mesh),
                    MeshMaterial3d(shirt_mat),
                    Transform::from_translation(Vec3::new(0.0, 0.975, 0.0)),
                ));

                // Left Arm
                root.spawn((
                    Mesh3d(arm_mesh.clone()),
                    MeshMaterial3d(pants_mat.clone()),
                    Transform::from_translation(Vec3::new(0.40, 0.975, 0.0)),
                ));

                // Right Arm
                root.spawn((
                    Mesh3d(arm_mesh),
                    MeshMaterial3d(pants_mat.clone()),
                    Transform::from_translation(Vec3::new(-0.40, 0.975, 0.0)),
                ));

                // Left Leg
                root.spawn((
                    Mesh3d(leg_mesh.clone()),
                    MeshMaterial3d(shoe_mat.clone()),
                    Transform::from_translation(Vec3::new(0.15, 0.325, 0.0)),
                ));

                // Right Leg
                root.spawn((
                    Mesh3d(leg_mesh),
                    MeshMaterial3d(shoe_mat),
                    Transform::from_translation(Vec3::new(-0.15, 0.325, 0.0)),
                ));

                // Name tag
                root.spawn((
                    RemotePlayerNameTag,
                    Text2d::new(event.name.clone()),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(0.0, 2.1, 0.0))
                        .with_scale(Vec3::splat(0.01)),
                ));
            });

        info!(
            "Spawned remote player '{}' (id={})",
            event.name, event.player_id
        );
    }
}

fn despawn_remote_player(
    mut commands: Commands,
    mut ev_leave: EventReader<PlayerLeaveEvent>,
    query: Query<(Entity, &RemotePlayer)>,
) {
    for event in ev_leave.read() {
        for (entity, remote) in &query {
            if remote.id == event.player_id {
                commands.entity(entity).despawn();
                info!("Despawned remote player (id={})", event.player_id);
            }
        }
    }
}

fn update_remote_players(
    time: Res<Time>,
    remote_states: Res<RemotePlayerStates>,
    mut query: Query<(&RemotePlayer, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (remote, mut transform) in &mut query {
        if let Some(target) = remote_states.players.get(&remote.id) {
            // Lerp position for smooth movement
            transform.translation = transform
                .translation
                .lerp(target.position, (REMOTE_LERP_SPEED * dt).min(1.0));

            // Apply yaw rotation
            transform.rotation = Quat::from_rotation_y(target.yaw);
        }
    }
}

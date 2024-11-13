use std::f32::consts::PI;

use bevy::{
    color::palettes::css::LIME,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PresentMode, PrimaryWindow},
};
use bevy_rapier3d::prelude::*;
use bevy_tnua::prelude::{
    TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController, TnuaControllerBundle, TnuaControllerPlugin,
};
use bevy_tnua_rapier3d::{TnuaRapier3dIOBundle, TnuaRapier3dPlugin, TnuaRapier3dSensorShape};

/// resource to control mouse locking
#[derive(Resource)]
struct MouseLocked(bool);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // optional: disable v-sync
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(MouseLocked(true))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(TnuaControllerPlugin::default())
        .add_plugins(TnuaRapier3dPlugin::default())
        .add_systems(Startup, (setup_scene, setup_player))
        .add_systems(Update, (player_rotation, update_player).chain())
        .add_systems(Update, toggle_mouse_lock)
        .add_systems(Update, mouse_lock)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerCamera(f32);

/// lock/unlock mouse based on MouseLocked resource
fn mouse_lock(locked: Res<MouseLocked>, mut window: Query<&mut Window, With<PrimaryWindow>>) {
    if locked.is_changed() {
        let mut window = window.single_mut();
        (window.cursor.grab_mode, window.cursor.visible) = if locked.0 {
            (CursorGrabMode::Confined, true)
        } else {
            (CursorGrabMode::None, false)
        };
    }
}

/// setup scene: a simple plane
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::new(10.0, 10.0))),
            material: materials.add(StandardMaterial::default()),
            ..default()
        })
        .insert(RigidBody::Fixed)
        .insert(Collider::cuboid(10.0, 0.1, 10.0));

    // a light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.0,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });
}

/// setup player entity (including child camera)
fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(Player)
        .insert(PbrBundle {
            mesh: meshes.add(Capsule3d::new(0.5, 1.0)),
            material: materials.add(StandardMaterial::from_color(LIME)),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(Collider::capsule(
            Vec3::new(0.0, -0.5, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
            0.5,
        ))
        .insert(TnuaControllerBundle::default())
        .insert(TnuaRapier3dIOBundle::default())
        .insert(TnuaRapier3dSensorShape(Collider::cylinder(0.0, 0.49)))
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Transform {
            translation: Vec3::new(0.0, 10.0, 0.0),
            ..default()
        })
        .with_children(|children| {
            children
                .spawn(Camera3dBundle {
                    transform: Transform {
                        translation: Vec3::new(0.0, 0.5, 0.0),
                        ..default()
                    },
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: PI * 0.5,
                        ..default()
                    }),
                    ..default()
                })
                .insert(PlayerCamera(0.0));
        });
}

/// listen for escape key to toggle mouse lock
fn toggle_mouse_lock(keyboard: Res<ButtonInput<KeyCode>>, mut exit: ResMut<MouseLocked>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.0 = !exit.0;
    }
}

/// determine inputs and move tnua controller
fn update_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut TnuaController, &Transform), With<Player>>,
) {
    let (mut controller, transform) = query.single_mut();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += Vec3::X;
    }

    // transform direction to correspond to camera rotation
    direction = (transform.rotation * direction) * Vec3::new(1.0, 0.0, 1.0);

    // set controller basis
    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction.normalize_or_zero() * 10.0,
        float_height: 1.5,
        ..default()
    });

    // add jump action if we're holding space
    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 4.0,
            shorten_extra_gravity: 0.0,
            ..default()
        });
    }
}

/// rotate the player entity by mouse X, but the camera by mouse Y
fn player_rotation(
    locked: Res<MouseLocked>,
    mut er_motion: EventReader<MouseMotion>,
    mut player_transform: Query<&mut Transform, With<Player>>,
    mut camera_transform: Query<
        (&mut Transform, &mut PlayerCamera),
        (With<Camera3d>, Without<Player>),
    >,
) {
    const SENS: f32 = 0.005;

    if !locked.0 {
        return;
    }

    let mut player_transform = player_transform.single_mut();
    let (mut camera_transform, mut player_camera) = camera_transform.single_mut();

    for ev in er_motion.read() {
        player_transform.rotate_y(-ev.delta.x * SENS);

        player_camera.0 = (player_camera.0 - ev.delta.y * SENS).clamp(-PI / 2.0, PI / 2.0);
        camera_transform.rotation = Quat::from_rotation_x(player_camera.0);
    }
}

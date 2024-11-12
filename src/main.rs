use std::f32::consts::PI;

use bevy::{
    color::palettes::css::LIME,
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_rapier3d::prelude::*;
use bevy_tnua::prelude::{
    TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController, TnuaControllerBundle, TnuaControllerPlugin,
};
use bevy_tnua_rapier3d::{TnuaRapier3dIOBundle, TnuaRapier3dPlugin, TnuaRapier3dSensorShape};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(TnuaControllerPlugin::default())
        .add_plugins(TnuaRapier3dPlugin::default())
        .add_systems(Startup, (setup_window, setup_scene, setup_player))
        .add_systems(Update, (player_rotation, update_player).chain())
        .add_systems(Update, window_close_listener)
        .run();
}

#[derive(Component)]
struct Player;

/// lock and hide cursor
fn setup_window(mut window: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window.single_mut();
    window.cursor.grab_mode = CursorGrabMode::Locked;
    window.cursor.visible = false;
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
            children.spawn(Camera3dBundle {
                transform: Transform {
                    translation: Vec3::new(0.0, 0.5, 0.0),
                    ..default()
                },
                projection: Projection::Perspective(PerspectiveProjection {
                    fov: PI * 0.5,
                    ..default()
                }),
                ..default()
            });
        });
}

// listen for escape key to close game
fn window_close_listener(keyboard: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
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

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction.normalize_or_zero() * 10.0,
        float_height: 1.5,
        ..default()
    });

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
    mut er_motion: EventReader<MouseMotion>,
    mut player_transform: Query<&mut Transform, With<Player>>,
    mut camera_transform: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    const SENS: f32 = 0.005;
    for ev in er_motion.read() {
        player_transform.single_mut().rotate_y(-ev.delta.x * SENS);
        camera_transform.single_mut().rotate_x(-ev.delta.y * SENS);
    }
}

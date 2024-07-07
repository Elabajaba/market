use std::{f32::consts::PI, time::Instant};

use bevy::{math::vec2, prelude::*, window::PrimaryWindow};
use fastrand::Rng;
use vleue_navigator::{
    prelude::{NavMeshUpdateMode, PrimitiveObstacle},
    NavMesh,
};

use crate::{
    agent3d::{spawn_agents, Navigator},
    ChangedMesh, Materials, MyCapsule, MyGroundPlane, Navmeshes,
};

pub struct SpawnerPlugin;

impl Plugin for SpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnedUnits { count: 0 })
            .add_systems(Update, (spawn_units, spawn_obstacle));
    }
}

#[derive(Resource)]
struct SpawnedUnits {
    count: u32,
}

fn spawn_units(
    commands: Commands,
    materials: Res<Materials>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&Handle<NavMesh>>,
    input: Res<ButtonInput<KeyCode>>,
    mut spawned_units: ResMut<SpawnedUnits>,
    capsule: Res<MyCapsule>,
) {
    if input.just_pressed(KeyCode::KeyP) {
        let count = 10000;
        let Ok(navmesh_id) = navmesh.get_single() else {
            return;
        };
        let Some(navmesh) = navmeshes.get(navmesh_id) else {
            return;
        };
        spawn_agents(
            commands,
            materials.into_inner(),
            &capsule.handle,
            navmesh,
            count,
        );
        spawned_units.count += count;
        info!(
            "Spawned {count} units, Total Units: {}",
            spawned_units.count
        );
    }
}

fn spawn_obstacle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // navmeshes: Res<Assets<NavMesh>>,
    // navmesh: Query<&Handle<NavMesh>>,
    input: Res<ButtonInput<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform)>,
    // query to get ground plane's transform
    q_plane: Query<&GlobalTransform, With<MyGroundPlane>>,
    mut changed_mesh: ResMut<ChangedMesh>,
    mut navmesh_update: Query<&mut NavMeshUpdateMode>,
) {
    if input.just_pressed(MouseButton::Right) {
        // let Ok(navmesh_id) = navmesh.get_single() else {
        //     return;
        // };
        // let Some(navmesh) = navmeshes.get(navmesh_id) else {
        //     return;
        // };

        let mut rng = Rng::new();

        let (camera, camera_transform) = q_camera.single();
        let ground_transform = q_plane.single();
        let window = q_window.single();
        let Some(cursor_position) = window.cursor_position() else {
            return;
        };
        let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            // if it was impossible to compute for whatever reason; we can't do anything
            return;
        };
        let Some(distance) = ray.intersect_plane(
            ground_transform.translation(),
            InfinitePlane3d::new(ground_transform.up()),
        ) else {
            return;
        };
        let global_cursor = ray.get_point(distance);

        let transform = Transform::from_translation(global_cursor);

        println!("Spawning obstacle at {:?}", transform);
        commands.spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0))),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            transform,
            ..default()
        });
        changed_mesh.changed = true;

        new_obstacle(&mut commands, &mut rng, transform);
        if let Ok(mut navmesh_update) = navmesh_update.get_single_mut() {
            *navmesh_update = NavMeshUpdateMode::OnDemand(true);
        }
    }
}

pub fn new_obstacle(commands: &mut Commands, rng: &mut Rng, transform: Transform) {
    commands.spawn((
        match rng.u32(0..8) {
            0 => PrimitiveObstacle::Rectangle(Rectangle {
                half_size: vec2(rng.f32() * 4.0 + 1.0, rng.f32() * 4.0 + 1.0),
            }),
            1 => PrimitiveObstacle::Circle(Circle {
                radius: rng.f32() * 4.0 + 1.0,
            }),
            2 => PrimitiveObstacle::Ellipse(Ellipse {
                half_size: vec2(rng.f32() * 4.0 + 1.0, rng.f32() * 4.0 + 1.0),
            }),
            3 => PrimitiveObstacle::CircularSector(CircularSector::new(
                rng.f32() * 3.5 + 1.5,
                rng.f32() * (PI - 0.5) + 0.5,
            )),
            4 => PrimitiveObstacle::CircularSegment(CircularSegment::new(
                rng.f32() * 3.5 + 1.5,
                rng.f32() * (PI - 1.) + 1.,
            )),
            5 => PrimitiveObstacle::Capsule(Capsule2d::new(
                rng.f32() * 2. + 1.,
                rng.f32() * 3.5 + 1.5,
            )),
            6 => PrimitiveObstacle::RegularPolygon(RegularPolygon::new(
                rng.f32() * 4.0 + 1.0,
                rng.usize(3..8),
            )),
            7 => PrimitiveObstacle::Rhombus(Rhombus::new(rng.f32() * 3. + 3., rng.f32() + 2.)),
            _ => unreachable!(),
        },
        transform,
        GlobalTransform::default(),
    ));
}

use std::f32::consts::PI;

use agent3d::MovementPlugin;
use bevy::{
    color::palettes,
    core_pipeline::Skybox,
    math::vec2,
    pbr::{wireframe::Wireframe, NotShadowCaster},
    prelude::*,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use fastrand::Rng;
use spawner::SpawnerPlugin;
use vleue_navigator::{
    prelude::{
        NavMeshBundle, NavMeshSettings, NavMeshUpdateMode, NavMeshUpdateModeBlocking,
        NavmeshUpdaterPlugin, PrimitiveObstacle,
    },
    NavMesh, Triangulation, VleueNavigatorPlugin,
};

mod agent3d;
mod camera_controller;
mod spawner;

#[derive(Resource)]
struct Navmeshes {
    navmesh: Handle<NavMesh>,
}

#[derive(Resource)]
pub struct Materials {
    unit_materials: Vec<Handle<StandardMaterial>>,
}

pub(crate) const MAP_SIZE: (f32, f32) = (2000., 2000.);

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Market".to_string(),
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                resolution: Vec2::new(1920., 1080.).into(),
                ..default()
            }),
            ..default()
        }),
        VleueNavigatorPlugin,
        // Auto update the navmesh.
        // Obstacles will be entities with the `Obstacle` marker component,
        // and use the `Aabb` component as the obstacle data source.
        NavmeshUpdaterPlugin::<PrimitiveObstacle>::default(),
        CameraControllerPlugin,
        SpawnerPlugin,
        MovementPlugin,
    ))
    .insert_resource(ChangedMesh {
        changed: true,
        old_entity: None,
        show: false,
    })
    .add_systems(PreUpdate, debug_navmesh)
    .add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let camera_controller = CameraController {
        ..Default::default()
    };

    let skybox_handle = asset_server.load("Ryfjallet_cubemap_bc7.ktx2");

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0., 200., 50.))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        camera_controller,
        Skybox {
            image: skybox_handle,
            brightness: 1000.0,
        },
    ));

    let half_size = Vec2::new(MAP_SIZE.0 / 2.0, MAP_SIZE.1 / 2.0);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Y, half_size)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            ..default()
        },
        MyGroundPlane,
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..default()
    });

    let mut unit_materials = Vec::new();
    for colour in COLOURS.iter() {
        unit_materials.push(materials.add(*colour));
    }

    commands.insert_resource(Materials { unit_materials });
    commands.insert_resource(MyCapsule {
        handle: meshes.add(Capsule3d::new(0.6, 1.75).mesh()),
    });

    let mut rng = Rng::new();
    rng.seed(437894728948239);
    let obstacles = (0..5000).into_iter().map(|_| {
        let x = rng.f32() * MAP_SIZE.0 - MAP_SIZE.0 / 2.0;
        let z = rng.f32() * MAP_SIZE.1 - MAP_SIZE.1 / 2.0;
        let mesh = match rng.u32(0..8) {
            0 => Rectangle {
                half_size: vec2(rng.f32() * 4.0 + 1.0, rng.f32() * 4.0 + 1.0),
            }.mesh().build(),
            1 => Circle {
                radius: rng.f32() * 4.0 + 1.0,
            }.mesh().build(),
            2 => Ellipse {
                half_size: vec2(rng.f32() * 4.0 + 1.0, rng.f32() * 4.0 + 1.0),
            }.mesh().build(),
            3 => CircularSector::new(
                rng.f32() * 3.5 + 1.5,
                rng.f32() * (PI - 0.5) + 0.5,
            ).mesh().build(),
            4 => CircularSegment::new(
                rng.f32() * 3.5 + 1.5,
                rng.f32() * (PI - 1.) + 1.,
            ).mesh().build(),
            5 => Capsule2d::new(
                rng.f32() * 2. + 1.,
                rng.f32() * 3.5 + 1.5,
            ).mesh().build(),
            6 => RegularPolygon::new(
                rng.f32() * 4.0 + 1.0,
                rng.usize(3..8),
            ).mesh().build(),
            7 => Rhombus::new(rng.f32() * 3. + 3., rng.f32() + 2.).mesh().build(),
            _ => unreachable!(),
        };
    });
    for _ in 0..5000 {
        let x = rng.f32() * MAP_SIZE.0 - MAP_SIZE.0 / 2.0;
        let z = rng.f32() * MAP_SIZE.1 - MAP_SIZE.1 / 2.0;
        let transform = Transform::from_translation(Vec3::new(x, 0.0, z));
    }

    let mut fixed = Triangulation::from_outer_edges(&vec![
        vec2(-half_size.x, -half_size.y),
        vec2(half_size.x, -half_size.y),
        vec2(half_size.x, half_size.y),
        vec2(-half_size.x, half_size.y),
    ]);
    // fixed.add_obstacles(obstacles.into_iter());

    // Spawn a new navmesh that will be automatically updated.
    commands.spawn((
        NavMeshBundle {
            settings: NavMeshSettings {
                // Define the outer borders of the navmesh.
                fixed,
                // Starting with a small mesh simplification factor to avoid very small geometry.
                // Small geometry can make navmesh generation fail due to rounding errors.
                // This example has round obstacles which can create small details.
                simplify: 0.,
                // default_delta: 0.4,
                merge_steps: 0,
                ..default()
            },
            // Mark it for update as soon as obstacles are changed.
            // Other modes can be debounced or manually triggered.
            update_mode: NavMeshUpdateMode::OnDemand(true),
            transform: Transform::from_rotation(Quat::from_rotation_x(PI / 2.0)),
            ..default()
        },
        NavMeshUpdateModeBlocking,
    ));

    let mut rng = Rng::new();
    rng.seed(437894728948239);

    for _ in 0..1000 {
        let x = rng.f32() * MAP_SIZE.0 - MAP_SIZE.0 / 2.0;
        let z = rng.f32() * MAP_SIZE.1 - MAP_SIZE.1 / 2.0;
        let transform = Transform::from_translation(Vec3::new(x, 0.0, z));
        spawner::new_obstacle(&mut commands, &mut rng, transform);
    }
}

#[derive(Component)]
pub struct MyGroundPlane;

#[derive(Resource)]
pub struct MyCapsule {
    pub handle: Handle<Mesh>,
}

const COLOURS: [Color; 18] = [
    Color::Srgba(palettes::tailwind::AMBER_400),
    Color::Srgba(palettes::tailwind::BLUE_400),
    Color::Srgba(palettes::tailwind::CYAN_400),
    Color::Srgba(palettes::tailwind::EMERALD_400),
    Color::Srgba(palettes::tailwind::FUCHSIA_400),
    Color::Srgba(palettes::tailwind::GREEN_400),
    Color::Srgba(palettes::tailwind::INDIGO_400),
    Color::Srgba(palettes::tailwind::LIME_400),
    Color::Srgba(palettes::tailwind::ORANGE_400),
    Color::Srgba(palettes::tailwind::PINK_400),
    Color::Srgba(palettes::tailwind::PURPLE_400),
    Color::Srgba(palettes::tailwind::RED_400),
    Color::Srgba(palettes::tailwind::ROSE_400),
    Color::Srgba(palettes::tailwind::SKY_400),
    Color::Srgba(palettes::tailwind::STONE_400),
    Color::Srgba(palettes::tailwind::TEAL_400),
    Color::Srgba(palettes::tailwind::VIOLET_400),
    Color::Srgba(palettes::tailwind::YELLOW_400),
];

#[derive(Resource)]
struct ChangedMesh {
    changed: bool,
    old_entity: Option<Entity>,
    show: bool,
}

fn debug_navmesh(
    mut commands: Commands,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&Handle<NavMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut changed_mesh: ResMut<ChangedMesh>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::BracketLeft) {
        changed_mesh.show = !changed_mesh.show;
        changed_mesh.changed = true;
    }
    if input.just_pressed(KeyCode::KeyN) {
        // force refresh the displayed debug navmesh
        changed_mesh.changed = true;
    }

    if !changed_mesh.show {
        if changed_mesh.old_entity.is_some() {
            commands
                .entity(changed_mesh.old_entity.unwrap())
                .despawn_recursive();
            changed_mesh.old_entity = None;
        }
        return;
    }

    let Ok(navmesh_id) = navmesh.get_single() else {
        return;
    };
    let Some(navmesh) = navmeshes.get(navmesh_id) else {
        return;
    };

    if changed_mesh.changed == false {
        return;
    }

    changed_mesh.changed = false;

    if let Some(old_entity) = changed_mesh.old_entity {
        commands.entity(old_entity).despawn_recursive();
    }

    let mesh = navmesh.to_wireframe_mesh();

    let entity = commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(Color::WHITE),
                transform: Transform::from_translation(Vec3::new(0.0, 0.1, 0.0)),
                // transform: Transform::from_translation(Vec3::new(
                //     -(MAP_SIZE.0 as f32) / 2.0,
                //     0.1,
                //     -(MAP_SIZE.1 as f32) / 2.0,
                // )),
                ..default()
            },
            NotShadowCaster,
            Wireframe,
        ))
        .id();
    changed_mesh.old_entity = Some(entity);
}

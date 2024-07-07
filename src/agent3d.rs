use bevy::{prelude::*, utils::EntityHashMap};
use fastrand::Rng;
use vleue_navigator::prelude::*;

use crate::{Materials, MAP_SIZE};

const MOVEMENT_SPEED: f32 = 8.0;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (new_paths, give_target_to_navigator, move_navigator).chain(),
        );
    }
}

#[derive(Component)]
pub struct Navigator {
    speed: f32,
    // color: Color,
}

#[derive(Component)]
pub struct Path {
    current: Vec3,
    next: Vec<Vec3>,
}

pub fn spawn_agents(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    my_materials: &Materials,
    capsule: &Handle<Mesh>,
    navmesh: &NavMesh,
    count: u32,
) {
    let mut rng = Rng::new();
    for i in 0..count {
        let transform = loop {
            let transform = Transform::from_translation(Vec3::new(
                rng.f32() * MAP_SIZE.0 - MAP_SIZE.0 / 2.0,
                1.75,
                rng.f32() * MAP_SIZE.1 - MAP_SIZE.1 / 2.0,
            ));
            if navmesh.transformed_is_in_mesh(transform.translation) {
                break transform;
            }
        };

        let material =
            my_materials.unit_materials[i as usize % my_materials.unit_materials.len()].clone();
        commands.spawn((
            PbrBundle {
                mesh: capsule.clone(),
                material,
                transform,
                ..default()
            },
            Navigator {
                speed: MOVEMENT_SPEED,
                // color: colour,
            },
        ));
    }
}

fn new_paths(
    mut commands: Commands,
    navigators: Query<Entity, With<Path>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyK) {
        for entity in &navigators {
            commands.entity(entity).remove::<Path>();
        }
    }
}

pub fn give_target_to_navigator(
    mut commands: ParallelCommands,
    navigators: Query<(Entity, &Transform), (With<Navigator>, Without<Path>)>,
    navmeshes: Res<Assets<NavMesh>>,
    navmesh: Query<&Handle<NavMesh>>,
    // mut deltas: Local<EntityHashMap<Entity, f32>>,
) {
    // let mut rng = Rng::new();
    let Ok(navmesh_id) = navmesh.get_single() else {
        return;
    };
    let Some(navmesh) = navmeshes.get(navmesh_id) else {
        return;
    };
    // for (entity, transform) in &navigators {
    navigators.par_iter().for_each(|(entity, transform)| {
        let mut target;
        // let delta = if !navmesh.transformed_is_in_mesh(transform.translation) {
        //     let delta = deltas.entry(entity).or_insert(0.0);
        //     *delta = *delta + 0.1;
        //     *delta
        // } else {
        //     0.0
        // };
        // navmesh.set_delta(delta);

        loop {
            target = Vec3::new(
                fastrand::f32() * MAP_SIZE.0 - MAP_SIZE.0 / 2.0,
                1.75,
                fastrand::f32() * MAP_SIZE.1 - MAP_SIZE.1 / 2.0,
            );

            if navmesh.transformed_is_in_mesh(target) {
                break;
            }
        }

        if let Some(path) = navmesh.transformed_path(transform.translation, target) {
            if let Some((first, remaining)) = path.path.split_first() {
                let mut next = remaining.into_iter().cloned().collect::<Vec<_>>();
                next.reverse();
                commands.command_scope(|mut commands| {
                    commands.entity(entity).insert(Path {
                        current: *first,
                        next,
                    });
                });
            }
        }
    });
}

// pub fn refresh_path<const SIZE: u32, const X: u32, const Y: u32>(
//     mut commands: Commands,
//     mut navigator: Query<(Entity, &Transform, &mut Path), With<Navigator>>,
//     mut navmeshes: ResMut<Assets<NavMesh>>,
//     navmesh: Query<(&Handle<NavMesh>, Ref<NavMeshStatus>)>,
//     transforms: Query<&Transform>,
//     mut deltas: Local<EntityHashMap<Entity, f32>>,
// ) {
//     let (navmesh_handle, status) = navmesh.single();
//     if !status.is_changed() && deltas.is_empty() {
//         return;
//     }
//     let Some(navmesh) = navmeshes.get_mut(navmesh_handle) else {
//         return;
//     };

//     for (entity, transform, mut path) in &mut navigator {
//         let target = transforms.get(path.target).unwrap().translation;
//         navmesh.set_delta(0.0);
//         if !navmesh.transformed_is_in_mesh(transform.translation) {
//             let delta_for_entity = deltas.entry(entity).or_insert(0.0);
//             *delta_for_entity = *delta_for_entity + 0.1;
//             navmesh.set_delta(*delta_for_entity);
//             continue;
//         }
//         if !navmesh.transformed_is_in_mesh(target) {
//             commands.entity(path.target).despawn();
//             commands.entity(entity).remove::<Path>();
//             continue;
//         }

//         let Some(new_path) = navmesh.transformed_path(transform.translation, target) else {
//             commands.entity(path.target).despawn();
//             commands.entity(entity).remove::<Path>();
//             continue;
//         };
//         if let Some((first, remaining)) = new_path.path.split_first() {
//             let mut remaining = remaining.into_iter().cloned().collect::<Vec<_>>();
//             remaining.reverse();
//             path.current = *first;
//             path.next = remaining;
//             deltas.remove(&entity);
//         }
//     }
// }

pub fn move_navigator(
    commands: ParallelCommands,
    mut navigator: Query<(&mut Transform, &mut Path, Entity, &Navigator)>,
    time: Res<Time>,
) {
    // for (mut transform, mut path, entity, navigator) in navigator.iter_mut() {
    navigator
        .par_iter_mut()
        .for_each(|(mut transform, mut path, entity, navigator)| {
            let mut temp_translation = transform.translation;
            temp_translation.y = 0.0;
            let move_direction = path.current - temp_translation;
            temp_translation +=
                move_direction.normalize() * time.delta_seconds() * navigator.speed;
            transform.translation.x = temp_translation.x;
            transform.translation.z = temp_translation.z;
            while temp_translation.distance(path.current) < navigator.speed / 50.0 {
                if let Some(next) = path.next.pop() {
                    path.current = next;
                } else {
                    commands.command_scope(|mut commands| {
                        commands.entity(entity).remove::<Path>();
                    });
                    break;
                }
            }
        });
}

pub fn display_navigator_path(
    navigator: Query<(&Transform, &Path, &Navigator)>,
    mut gizmos: Gizmos,
) {
    for (transform, path, _navigator) in &navigator {
        let mut to_display = path.next.clone();
        to_display.push(path.current.clone());
        to_display.push(transform.translation);
        to_display.reverse();
        if to_display.len() >= 1 {
            gizmos.linestrip(
                to_display.iter().map(|xz| Vec3::new(xz.x, 0.1, xz.z)),
                Color::WHITE,
            );
        }
    }
}

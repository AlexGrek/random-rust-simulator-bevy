// --- Example Concrete Data Types ---

use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    asset::Assets,
    color::{Color, palettes::css::LIMEGREEN},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        component::Component,
        query::{With, Without},
        system::{Commands, Local, Query, Res, ResMut},
    },
    gizmos::gizmos::Gizmos,
    input::{ButtonInput, keyboard::KeyCode},
    log::info,
    math::{Vec2, Vec3, primitives::Circle},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    time::Time,
    transform::components::{GlobalTransform, Transform},
};

use crate::{
    core::{
        basics::{
            DEFAULT_CHUNK_DIMENSION_TILES, DEFAULT_RENDER_DISTANCE_CHUNKS,
            GAME_WORLD_CENTER_THRESHOLD, Point, TILE_SIZE_IN_UNITS,
        },
        chunks::{
            ChunkCoords, DataChunk, DataMap, FlatGrid, GridData, MapDataProducer,
            data_map_load_unload_system, data_map_process_completed_tasks_system,
            data_map_spawn_tasks_system, insert_chunked_plugin,
        },
    },
    game::{
        Player,
        world::passability::{PassabilityProducer, check_player_passability},
    },
};

pub mod core;
pub mod game;

#[derive(Component)]
struct FollowCamera {
    pub smoothing: f32, // Higher values = smoother but slower following
    pub offset: Vec3,   // Optional offset from player position
}

impl Default for FollowCamera {
    fn default() -> Self {
        Self {
            smoothing: 2.0,
            offset: Vec3::ZERO,
        }
    }
}

fn setup_game(
    mut commands: Commands,
    mut passability_map: ResMut<DataMap<PassabilityProducer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d::default(), FollowCamera::default()));
    commands.spawn((
        Player,
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        GlobalTransform::default(),
        // Add visual for player
        Mesh2d(meshes.add(Circle::new(5.0))), // Circle directly from bevy::math
        MeshMaterial2d(materials.add(Color::from(LIMEGREEN))), // Explicitly create and add ColorMaterial
    ));
}

// System to visualize loaded chunks (optional, for debugging)
fn visualize_loaded_chunks(mut gizmos: Gizmos, passability_map: Res<DataMap<PassabilityProducer>>) {
    for (coords, _) in passability_map.loaded_chunks.iter() {
        let chunk_world_pos = coords.to_world_pos(passability_map.chunk_size_units);
        let center = chunk_world_pos + Vec2::splat(passability_map.chunk_size_units / 2.0);
        let size = Vec2::splat(passability_map.chunk_size_units);
        gizmos.rect_2d(Vec2::ZERO, size, Color::srgba(0.0, 1.0, 0.0, 0.1)); // Semi-transparent green
    }
}

// System to visualize requested chunks (optional, for debugging)
fn visualize_requested_chunks(
    mut gizmos: Gizmos,
    passability_map: Res<DataMap<PassabilityProducer>>,
) {
    for coords in passability_map.requested_chunks.iter() {
        let chunk_world_pos = coords.to_world_pos(passability_map.chunk_size_units);
        let center = chunk_world_pos + Vec2::splat(passability_map.chunk_size_units / 2.0);
        let size = Vec2::splat(passability_map.chunk_size_units);
        gizmos.rect_2d(Vec2::ZERO, size, Color::srgba(1.0, 1.0, 0.0, 0.2)); // Semi-transparent yellow
    }
    for (coords, _) in passability_map.pending_tasks.iter() {
        let chunk_world_pos = coords.to_world_pos(passability_map.chunk_size_units);
        let center = chunk_world_pos + Vec2::splat(passability_map.chunk_size_units / 2.0);
        let size = Vec2::splat(passability_map.chunk_size_units);
        gizmos.rect_2d(Vec2::ZERO, size, Color::srgba(1.0, 0.5, 0.0, 0.2)); // Semi-transparent orange
    }
}

// Example: System to read passability for player's current tile

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_game)
        // Insert your DataMap resources
        .insert_resource(DataMap::<PassabilityProducer>::new(
            PassabilityProducer,
            DEFAULT_CHUNK_DIMENSION_TILES,
            DEFAULT_RENDER_DISTANCE_CHUNKS,
        ))
        // Add systems to the Update schedule
        .add_systems(
            Update,
            (
                game::player_movement,
                // These run for each DataMap type
                // Add these lines for each additional DataMap you create (e.g., TileTypeProducer)
                // data_map_load_unload_system::<TileTypeProducer>,
                // data_map_spawn_tasks_system::<TileTypeProducer>,
                // data_map_process_completed_tasks_system::<TileTypeProducer>,

                // Game logic systems
                check_player_passability,
                visualize_loaded_chunks,    // Debug visualization
                visualize_requested_chunks, // Debug visualization
                // Camera
                camera_follow_system,
            ),
        );
    insert_chunked_plugin(&mut app, PassabilityProducer, 50);
    app.run();
}

pub fn camera_follow_system(
    player_query: Query<&Transform, (With<Player>, Without<FollowCamera>)>,
    mut camera_query: Query<(&mut Transform, &FollowCamera), Without<Player>>,
    time: Res<Time>,
) {
    if let Ok(player_transform) = player_query.single() {
        for (mut camera_transform, follow_camera) in camera_query.iter_mut() {
            let target_position = player_transform.translation + follow_camera.offset;

            // Smooth interpolation using exponential decay
            let smoothing_factor = 1.0 - (-follow_camera.smoothing * time.delta_secs()).exp();

            camera_transform.translation = camera_transform
                .translation
                .lerp(target_position, smoothing_factor);
        }
    }
}

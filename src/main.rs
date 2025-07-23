// --- Example Concrete Data Types ---

use bevy::{
    app::{App, Startup, Update}, asset::{Assets, Handle}, color::{palettes::css::{LIMEGREEN, RED}, Color}, core_pipeline::core_2d::Camera2d, ecs::{
        component::Component, query::{With, Without}, resource::Resource, system::{Commands, Query, Res, ResMut}
    }, gizmos::gizmos::Gizmos, math::{primitives::Circle, Vec2, Vec3}, platform::collections::{HashMap, HashSet}, render::mesh::{Mesh, Mesh2d}, sprite::{ColorMaterial, MeshMaterial2d}, time::{Fixed, Time}, transform::components::{GlobalTransform, Transform}, DefaultPlugins
};

use crate::{
    core::{
        basics::DEFAULT_RENDER_DISTANCE_CHUNKS,
        chunks::{insert_chunked_plugin, DataMap}, constants::DEFAULT_CHUNK_DIMENSION_TILES,
    },
    game::{
        physix, render::{light_sim::lighting::Lighting, tilemap_render::{
            background_load_required_chunks_system, background_load_unload_system, BackgroundHypertileTracker,
        }}, world::passability::{check_player_passability, PassabilityProducer}, MapRevealActor, Player
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

#[derive(Debug, Default, Clone, Resource)]
pub struct Pallete {
    pub colors: HashMap::<String, Handle<ColorMaterial>>
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut pallete: ResMut<Pallete>,
    
) {
    commands.spawn((Camera2d::default(), FollowCamera::default()));
    let limegreen = materials.add(ColorMaterial::from_color(Color::from(LIMEGREEN)));
    let red = materials.add(ColorMaterial::from_color(Color::from(RED)));
    pallete.colors.insert("limegreen".to_string(), limegreen);
    pallete.colors.insert("red".to_string(), red);
    commands.spawn((
        Player,
        MapRevealActor,
        crate::game::physix::PrevXY::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        GlobalTransform::default(),
        // Add visual for player
        Mesh2d(meshes.add(Circle::new(5.0))), // Circle directly from bevy::math
        MeshMaterial2d(pallete.colors.get("limegreen").unwrap().clone()), // Explicitly create and add ColorMaterial
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
        .add_systems(
            Update,
            (
                background_load_required_chunks_system,
                background_load_unload_system,
            ),
        )
        // Insert your DataMap resources
        .insert_resource(DataMap::<PassabilityProducer>::new(
            PassabilityProducer,
            DEFAULT_CHUNK_DIMENSION_TILES,
            DEFAULT_RENDER_DISTANCE_CHUNKS * 10,
        ))
        .insert_resource(BackgroundHypertileTracker {spawned: HashSet::new(), requested: HashSet::new()})
        .insert_resource(Pallete::default())
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        // Add systems to the Update schedule
        .add_systems(
            Update,
            (
                game::player_movement,
                physix::bounce_back,
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
    app.add_plugins(Lighting);
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

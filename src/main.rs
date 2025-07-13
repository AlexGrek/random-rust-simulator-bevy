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
    math::{Vec2, Vec3, VectorSpace, primitives::Circle},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    time::Time,
    transform::components::{GlobalTransform, Transform},
};

use crate::core::{
    basics::{
        ChunkCoords, DEFAULT_CHUNK_DIMENSION_TILES, DEFAULT_RENDER_DISTANCE_CHUNKS, DataChunk,
        FlatGrid, GAME_WORLD_CENTER_THRESHOLD, GridData, Point, TILE_SIZE_IN_UNITS,
    },
    data_map_producer::MapDataProducer,
    data_map_resource::DataMap,
    data_map_systems::{
        data_map_load_unload_system, data_map_process_completed_tasks_system,
        data_map_spawn_tasks_system,
    },
};

pub mod core;

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

// Passability
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Passability(pub u8);

impl Passability {
    pub const IMPASSABLE: Passability = Passability(0);
    pub const FREE: Passability = Passability(255);
}

// Passability DataProducer
#[derive(Default, Clone)]
pub struct PassabilityProducer;

impl MapDataProducer for PassabilityProducer {
    type Item = Passability;
    type GridType = FlatGrid<Passability>;

    fn default_value(&self) -> Self::Item {
        Passability::IMPASSABLE
    }

    fn generate_chunk(
        &self,
        coords: ChunkCoords,
        dimension_tiles: u32,
    ) -> DataChunk<Self::GridType> {
        let mut grid = FlatGrid::new(dimension_tiles, Passability::FREE);
        let chunk_center_x =
            coords.x as f32 * dimension_tiles as f32 + dimension_tiles as f32 / 2.0;
        let chunk_center_y =
            coords.y as f32 * dimension_tiles as f32 + dimension_tiles as f32 / 2.0;

        for y in 0..dimension_tiles {
            for x in 0..dimension_tiles {
                let world_tile_x = coords.x * dimension_tiles as isize + x as isize;
                let world_tile_y = coords.y * dimension_tiles as isize + y as isize;

                let dist_from_center =
                    ((world_tile_x as f32).powi(2) + (world_tile_y as f32).powi(2)).sqrt();

                // Make tiles impassable further from center
                let mut passability = Passability::FREE;
                if dist_from_center > GAME_WORLD_CENTER_THRESHOLD {
                    let falloff = (dist_from_center - GAME_WORLD_CENTER_THRESHOLD) / 500.0;
                    info!("Falloff: {}", &falloff);
                    passability = Passability((255.0 - (falloff * 255.0).min(255.0)) as u8);
                    if passability.0 < 250 {
                        passability = Passability(0);
                    }
                }

                // Example: Add a small "river" (impassable)
                if world_tile_y > 100 && world_tile_y < 105 {
                    passability = Passability::IMPASSABLE;
                }

                grid.set_item(x, y, passability);
            }
        }

        DataChunk { grid }
    }
}

// --- Player Component for focus point ---
#[derive(Component)]
struct Player;

// --- Example Player movement system ---
fn player_movement(
    mut player_query: Query<&mut Transform, With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let transform = player_query.single_mut();
    let move_speed = 200.0; // units per second
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        transform.unwrap().translation += direction.normalize() * move_speed * time.delta_secs();
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

    // Initialize the map around the center
    passability_map.init(50); // Initialize chunks within 50 tile Manhattan distance
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
fn check_player_passability(
    player_query: Query<&Transform, With<Player>>,
    mut passability_map: ResMut<DataMap<PassabilityProducer>>, // Needs mut to make requests
    mut last_checked_point: Local<Option<Point>>,
) {
    let player_transform = player_query.single().unwrap();
    let player_tile_point = Point {
        x: (player_transform.translation.x / TILE_SIZE_IN_UNITS).round() as isize,
        y: (player_transform.translation.y / TILE_SIZE_IN_UNITS).round() as isize,
    };

    if last_checked_point.map_or(true, |p| p != player_tile_point) {
        *last_checked_point = Some(player_tile_point);
        let passability = passability_map.get(player_tile_point); // This will request the chunk if not loaded
        info!(
            "Player at tile {:?} has passability: {:?}",
            player_tile_point, passability
        );

        // Example: Try writing
        if passability.0 == Passability::FREE.0 {
            // passability_map.write(player_tile_point + Point{x:1, y:0}, Passability::IMPASSABLE);
            // info!("Queued write to make tile {:?} impassable", player_tile_point + Point{x:1, y:0});
        }
    }
}

pub fn insert_chunked_plugin<P>(
    app: &mut bevy::prelude::App,
    producer: P,
) -> &mut bevy::prelude::App
where
    P: MapDataProducer + Send + Sync + Clone + 'static,
    <P as MapDataProducer>::GridType: Send + Sync,
    <P as MapDataProducer>::Item: Send + Copy + Default + Sync,
{
    app.insert_resource(DataMap::<P>::new(
        producer,
        DEFAULT_CHUNK_DIMENSION_TILES,
        DEFAULT_RENDER_DISTANCE_CHUNKS,
    ))
    .add_systems(
        Update,
        (
            data_map_load_unload_system::<P>,
            data_map_spawn_tasks_system::<P>,
            data_map_process_completed_tasks_system::<P>,
        ),
    )
}

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
                player_movement,
                // These run for each DataMap type
                (
                    data_map_load_unload_system::<PassabilityProducer>,
                    data_map_spawn_tasks_system::<PassabilityProducer>,
                    data_map_process_completed_tasks_system::<PassabilityProducer>,
                ),
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
    app.run();
}

fn camera_follow_system(
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

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use std::{fmt::Debug, hash::Hash};

use crate::{
    core::{basics::{
         Point, DEFAULT_RENDER_DISTANCE_CHUNKS
    }, constants::{DEFAULT_CHUNK_DIMENSION_TILES, TILE_SIZE_IN_UNITS}, units::{TilesCount}},
    game::MapRevealActor,
}; // For polling tasks

use std::sync::Arc;

use bevy::prelude::Vec3Swizzles;
use bevy::{
    ecs::{
        entity::Entity,
        query::With,
        system::{Commands, Query, ResMut},
    },
    log::info,
    transform::components::Transform,
};

use crate::Player;

/// Absolute chunk coordinates.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
pub struct ChunkCoords {
    pub x: isize,
    pub y: isize,
}

impl ChunkCoords {
    /// Converts a world tile `Point` to `ChunkCoords`.
    pub fn from_point(point: Point, chunk_dimension_tiles: TilesCount) -> Self {
        ChunkCoords {
            x: (point.x as f32 / chunk_dimension_tiles as f32).floor() as isize,
            y: (point.y as f32 / chunk_dimension_tiles as f32).floor() as isize,
        }
    }

    /// Converts a world unit `Vec2` to `ChunkCoords`.
    pub fn from_world_pos(pos: Vec2, chunk_size_units: f32) -> Self {
        ChunkCoords {
            x: (pos.x / chunk_size_units).floor() as isize,
            y: (pos.y / chunk_size_units).floor() as isize,
        }
    }

    /// Converts `ChunkCoords` to the world tile `Point` of its bottom-left corner.
    pub fn to_bottom_left_tile_point(&self, chunk_dimension_tiles: TilesCount) -> Point {
        Point {
            x: self.x * chunk_dimension_tiles as isize,
            y: self.y * chunk_dimension_tiles as isize,
        }
    }

    /// Converts `ChunkCoords` to the world unit `Vec2` of its bottom-left corner.
    pub fn to_world_pos(&self, chunk_size_units: f32) -> Vec2 {
        Vec2::new(
            self.x as f32 * chunk_size_units,
            self.y as f32 * chunk_size_units,
        )
    }
}

pub trait GridData: Send + Sync + 'static + Debug + Clone {
    type Item: Copy + Debug + Default; // Default trait required for new()
    fn dimension(&self) -> TilesCount;
    fn get_item(&self, x: TilesCount, y: TilesCount) -> Option<&Self::Item>;
    fn get_item_mut(&mut self, x: TilesCount, y: TilesCount) -> Option<&mut Self::Item>;
    fn set_item(&mut self, x: TilesCount, y: TilesCount, item: Self::Item) -> bool;
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
}

#[derive(Debug, Clone)]
pub struct FlatGrid<T>
where
    T: Copy + Debug + Send + Sync + 'static + Default,
{
    data: Vec<T>,
    dimension: TilesCount,
}

impl<T> FlatGrid<T>
where
    T: Copy + Debug + Send + Sync + 'static + Default,
{
    pub fn new(dimension: TilesCount, default_value: T) -> Self {
        let num_elements = (dimension * dimension) as usize;
        FlatGrid {
            data: vec![default_value; num_elements],
            dimension,
        }
    }

    fn calculate_index(&self, x: TilesCount, y: TilesCount) -> Option<usize> {
        if x < self.dimension && y < self.dimension {
            Some((y * self.dimension + x) as TilesCount)
        } else {
            None
        }
    }
}

impl<T> GridData for FlatGrid<T>
where
    T: Copy + Debug + Send + Sync + 'static + Default,
{
    type Item = T;

    fn dimension(&self) -> TilesCount {
        self.dimension
    }

    fn get_item(&self, x: TilesCount, y: TilesCount) -> Option<&Self::Item> {
        self.calculate_index(x, y).map(|idx| &self.data[idx])
    }

    fn get_item_mut(&mut self, x: TilesCount, y: TilesCount) -> Option<&mut Self::Item> {
        self.calculate_index(x, y).map(|idx| &mut self.data[idx])
    }

    fn set_item(&mut self, x: TilesCount, y: TilesCount, item: Self::Item) -> bool {
        if let Some(idx) = self.calculate_index(x, y) {
            self.data[idx] = item;
            true
        } else {
            false
        }
    }

    fn as_slice(&self) -> &[Self::Item] {
        &self.data
    }

    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        &mut self.data
    }
}

/// A chunk of specific map data. The manager knows its coordinates.
#[derive(Debug, Clone)]
pub struct DataChunk<T: GridData> {
    pub grid: T,
}

// Marker component for tasks in flight
#[derive(Component)]
pub struct ChunkGenTask<T: GridData>(pub Task<DataChunk<T>>);

pub trait MapDataProducer: Send + Sync + 'static + Clone {
    type Item: Copy + Default + Send + Sync;
    type GridType: GridData<Item = Self::Item> + Send + Sync;

    /// Returns the default value for an ungenerated tile.
    fn default_value(&self) -> Self::Item;

    /// Generates a chunk of data for the given coordinates.
    /// Returns the DataChunk asset.
    fn generate_chunk(
        &self,
        coords: ChunkCoords,
        dimension_tiles: TilesCount,
    ) -> DataChunk<Self::GridType>;
}

/// The central resource for managing a chunked map of type T.
#[derive(Resource)]
pub struct DataMap<P: MapDataProducer> {
    pub loaded_chunks: HashMap<ChunkCoords, DataChunk<P::GridType>>,
    pub requested_chunks: HashSet<ChunkCoords>,
    // Maps chunk coords to the entity holding the generation task
    pub pending_tasks: HashMap<ChunkCoords, Entity>,
    pub write_queue: HashMap<Point, P::Item>, // Writes to uncreated/unloaded cells
    pub producer: P,
    pub chunk_dimension_tiles: TilesCount,
    pub chunk_size_units: f32, // Derived from chunk_dimension_tiles and TILE_SIZE_IN_UNITS
    pub render_distance_chunks: usize, // Used by the map manager system to determine loading radius
}

impl<P: MapDataProducer> DataMap<P> {
    pub fn new(producer: P, chunk_dimension_tiles: TilesCount, render_distance_chunks: usize) -> Self {
        let chunk_size_units = chunk_dimension_tiles as f32 * TILE_SIZE_IN_UNITS;
        Self {
            loaded_chunks: HashMap::new(),
            requested_chunks: HashSet::new(),
            pending_tasks: HashMap::new(),
            write_queue: HashMap::new(),
            producer,
            chunk_dimension_tiles,
            chunk_size_units,
            render_distance_chunks,
        }
    }

    // --- Public API for Game Logic ---

    /// Requests and gets the data at a specific world tile Point.
    /// Spawns a chunk generation request if the chunk is not loaded.
    pub fn get(&mut self, point: Point) -> P::Item {
        // Check write queue first (acts as a cache for pending writes)
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return queued_value;
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.loaded_chunks.get(&chunk_coords) {
            let local_x = (point.x % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            let local_y = (point.y % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            chunk
                .grid
                .get_item(local_x as TilesCount, local_y as TilesCount)
                .copied() // Get a copy of the item
                .unwrap_or_else(|| self.producer.default_value()) // Should not happen if logic is correct
        } else {
            // Chunk not loaded, request it
            self.requested_chunks.insert(chunk_coords);
            self.producer.default_value() // Return default while waiting for generation
        }
    }

    /// Requests and gets the data at a specific floating-point world position.
    /// Rounds the position to the nearest tile center.
    /// Spawns a chunk generation request if the chunk is not loaded.
    pub fn get_rounded(&mut self, world_pos: Vec2) -> P::Item {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.get(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Attempts to get the data at a specific world tile Point.
    /// Returns `Some(T)` if the chunk is loaded, `None` otherwise.
    /// Spawns a chunk generation request if the chunk is not loaded.
    pub fn get_option(&mut self, point: Point) -> Option<P::Item> {
        // Check write queue first
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return Some(queued_value);
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.loaded_chunks.get(&chunk_coords) {
            let local_x = (point.x % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            let local_y = (point.y % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            chunk.grid.get_item(local_x as TilesCount, local_y as TilesCount).copied()
        } else {
            self.requested_chunks.insert(chunk_coords);
            None
        }
    }

    /// Attempts to get the data at a specific floating-point world position.
    /// Returns `Some(T)` if the chunk is loaded, `None` otherwise.
    /// Spawns a chunk generation request if the chunk is not loaded.
    pub fn get_rounded_option(&mut self, world_pos: Vec2) -> Option<P::Item> {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.get_option(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Reads the data at a specific world tile Point without spawning any generation requests.
    /// Returns `Some(T)` if the chunk is loaded, `None` otherwise.
    pub fn read(&self, point: Point) -> Option<P::Item> {
        // Check write queue first for potential cached writes
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return Some(queued_value);
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        self.loaded_chunks.get(&chunk_coords).and_then(|chunk| {
            let local_x = (point.x % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            let local_y = (point.y % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            chunk.grid.get_item(local_x as TilesCount, local_y as TilesCount).copied()
        })
    }

    /// Reads the data at a specific floating-point world position without spawning any generation requests.
    /// Returns `Some(T)` if the chunk is loaded, `None` otherwise.
    pub fn read_rounded(&self, world_pos: Vec2) -> Option<P::Item> {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.read(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Writes data to a specific world tile Point.
    /// If the chunk is loaded, the write is applied immediately.
    /// If not, the write is queued for when the chunk is generated.
    pub fn write(&mut self, point: Point, value: P::Item) {
        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.loaded_chunks.get_mut(&chunk_coords) {
            let local_x = (point.x % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            let local_y = (point.y % self.chunk_dimension_tiles as isize
                + self.chunk_dimension_tiles as isize)
                % self.chunk_dimension_tiles as isize;
            chunk.grid.set_item(local_x as TilesCount, local_y as TilesCount, value);
            // Remove from write queue if it was there and is now written
            self.write_queue.remove(&point);
        } else {
            // Chunk not loaded, queue the write
            self.write_queue.insert(point, value);
            // Also request the chunk if it's not already
            self.requested_chunks.insert(chunk_coords);
        }
    }

    /// Initializes chunks in a Manhattan distance radius from the center (0,0).
    /// This method is non-blocking and spawns generation requests.
    pub fn init(&mut self, manhattan_distance_tiles: usize) {
        let center_chunk = ChunkCoords { x: 0, y: 0 };
        let center_point = Point { x: 0, y: 0 };

        let chunk_manhattan_distance =
            (manhattan_distance_tiles as f32 / self.chunk_dimension_tiles as f32).ceil() as isize;

        for x_offset in -chunk_manhattan_distance..=chunk_manhattan_distance {
            for y_offset in -chunk_manhattan_distance..=chunk_manhattan_distance {
                let current_chunk_coords = ChunkCoords {
                    x: center_chunk.x + x_offset,
                    y: center_chunk.y + y_offset,
                };
                // Only request if not already loaded or pending
                if !self.loaded_chunks.contains_key(&current_chunk_coords)
                    && !self.pending_tasks.contains_key(&current_chunk_coords)
                {
                    self.requested_chunks.insert(current_chunk_coords);
                }
            }
        }
        // info!(
        //     "DataMap<{}> init requested chunks up to Manhattan distance {} (chunks: {})",
        //     std::any::type_name::<P::Item>(),
        //     manhattan_distance_tiles,
        //     chunk_manhattan_distance
        // );
    }
}

// System to manage loading/unloading based on a focus point (e.g., player/camera)
pub fn data_map_load_unload_system<P: MapDataProducer>(
    player_query: Query<&Transform, With<MapRevealActor>>,
    mut data_map: ResMut<DataMap<P>>,
) {
    for player_transform in player_query.as_readonly().iter() {
        let focus_world_pos = player_transform.translation.xy();
        let current_focus_chunk_coords =
            ChunkCoords::from_world_pos(focus_world_pos, data_map.chunk_size_units);

        let mut required_chunks_set: HashSet<ChunkCoords> = HashSet::new();

        for dx in
            -(data_map.render_distance_chunks as isize)..=(data_map.render_distance_chunks as isize)
        {
            for dy in -(data_map.render_distance_chunks as isize)
                ..=(data_map.render_distance_chunks as isize)
            {
                required_chunks_set.insert(ChunkCoords {
                    x: current_focus_chunk_coords.x + dx,
                    y: current_focus_chunk_coords.y + dy,
                });
            }
        }

        // Unload chunks that are no longer required
        // data_map
        //     .loaded_chunks
        //     .retain(|coords, _| required_chunks_set.contains(coords));

        // Request new chunks
        for coords in required_chunks_set.iter() {
            if !data_map.loaded_chunks.contains_key(coords) &&
           !data_map.pending_tasks.contains_key(coords) && // Don't request if already pending
           !data_map.requested_chunks.contains(coords)
            // Don't request if already in queue
            {
                data_map.requested_chunks.insert(*coords);
            }
        }
    }
}

pub fn data_map_load_unload_system_for_player<P: MapDataProducer>(
    player_query: Query<&Transform, With<Player>>,
    mut data_map: ResMut<DataMap<P>>,
) {
    for player_transform in player_query.as_readonly().iter() {
        let focus_world_pos = player_transform.translation.xy();
        let current_focus_chunk_coords =
            ChunkCoords::from_world_pos(focus_world_pos, data_map.chunk_size_units);

        let mut required_chunks_set: HashSet<ChunkCoords> = HashSet::new();

        for dx in
            -(data_map.render_distance_chunks as isize)..=(data_map.render_distance_chunks as isize)
        {
            for dy in -(data_map.render_distance_chunks as isize)
                ..=(data_map.render_distance_chunks as isize)
            {
                required_chunks_set.insert(ChunkCoords {
                    x: current_focus_chunk_coords.x + dx,
                    y: current_focus_chunk_coords.y + dy,
                });
            }
        }

        // Unload chunks that are no longer required
        data_map
            .loaded_chunks
            .retain(|coords, _| required_chunks_set.contains(coords));

        // Request new chunks
        for coords in required_chunks_set.iter() {
            if !data_map.loaded_chunks.contains_key(coords) &&
           !data_map.pending_tasks.contains_key(coords) && // Don't request if already pending
           !data_map.requested_chunks.contains(coords)
            // Don't request if already in queue
            {
                data_map.requested_chunks.insert(*coords);
            }
        }
    }
}

// System to spawn background tasks for requested chunks
pub fn data_map_spawn_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut data_map: ResMut<DataMap<P>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    let mut new_pending_tasks = Vec::new(); // Collect tasks to add to pending_tasks map

    let producer = Arc::new(data_map.producer.clone());

    for coords in data_map.requested_chunks.iter() {
        if !data_map.pending_tasks.contains_key(coords) {
            // Double check for safety
            let chunk_dimension = data_map.chunk_dimension_tiles;
            let current_coords = *coords;
            let pr = producer.clone();

            let task = thread_pool
                .spawn(async move { pr.generate_chunk(current_coords, chunk_dimension) });

            let task_entity = commands
                .spawn((
                    current_coords, // Attach coords for easy lookup by completion system
                    ChunkGenTask(task),
                ))
                .id();

            new_pending_tasks.push((current_coords, task_entity));
            // info!(
            //     "Spawned DataMap<{}> gen task for chunk: {:?}",
            //     std::any::type_name::<P::Item>(),
            //     current_coords
            // );
        }
    }

    // Add new pending tasks to the map and clear requested chunks
    for (coords, entity) in new_pending_tasks {
        data_map.pending_tasks.insert(coords, entity);
    }
    data_map.requested_chunks.clear();
}

// System to process completed background tasks
pub fn data_map_process_completed_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut query: Query<(Entity, &ChunkCoords, &mut ChunkGenTask<P::GridType>)>,
    mut data_map: ResMut<DataMap<P>>,
) {
    let mut completed_chunks = Vec::new();

    for (task_entity, coords, mut gen_task) in query.iter_mut() {
        if let Some(generated_chunk) = future::block_on(future::poll_once(&mut gen_task.0)) {
            completed_chunks.push((*coords, generated_chunk));
            commands.entity(task_entity).despawn(); // Remove the temporary task entity
            data_map.pending_tasks.remove(coords); // Remove from pending map
        }
    }

    // Apply completed chunks and pending writes
    for (coords, mut chunk) in completed_chunks {
        // info!(
        //     "DataMap<{}> chunk {:?} generated.",
        //     std::any::type_name::<P::Item>(),
        //     coords
        // );
        // Apply any writes from the queue to this newly generated chunk
        let chunk_bottom_left_tile =
            coords.to_bottom_left_tile_point(data_map.chunk_dimension_tiles);

        // Use a temporary Vec to collect points to remove from write_queue
        let mut points_to_remove = Vec::new();

        let chunk_dimension_tiles = data_map.chunk_dimension_tiles;

        // Iterate over the write_queue and identify items to apply
        for (&point, &value) in data_map.write_queue.iter() {
            if point.x >= chunk_bottom_left_tile.x
                && point.x < chunk_bottom_left_tile.x + chunk_dimension_tiles as isize
                && point.y >= chunk_bottom_left_tile.y
                && point.y < chunk_bottom_left_tile.y + chunk_dimension_tiles as isize
            {
                // This write belongs to the newly generated chunk
                let local_x = (point.x % chunk_dimension_tiles as isize
                    + chunk_dimension_tiles as isize)
                    % chunk_dimension_tiles as isize;
                let local_y = (point.y % chunk_dimension_tiles as isize
                    + chunk_dimension_tiles as isize)
                    % chunk_dimension_tiles as isize;
                chunk.grid.set_item(local_x as TilesCount, local_y as TilesCount, value);
                points_to_remove.push(point); // Mark for removal
            }
        }

        // Now, remove the marked points from the write_queue outside the iteration
        for point in points_to_remove {
            data_map.write_queue.remove(&point);
        }

        data_map.loaded_chunks.insert(coords, chunk);
    }
}

pub fn insert_chunked_plugin<P>(
    app: &mut bevy::prelude::App,
    producer: P,
    manhattan_distance_tiles_init: TilesCount,
) -> &mut bevy::prelude::App
where
    P: MapDataProducer + Send + Sync + Clone + 'static,
    <P as MapDataProducer>::GridType: Send + Sync,
    <P as MapDataProducer>::Item: Send + Copy + Default + Sync,
{
    app.add_systems(Startup, move |mut map: ResMut<DataMap<P>>| {
        map.init(manhattan_distance_tiles_init)
    });
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

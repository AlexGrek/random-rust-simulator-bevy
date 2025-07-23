use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use futures_lite::future;
use std::mem;

use crate::{
    core::{
        basics::{DEFAULT_RENDER_DISTANCE_CHUNKS, Point},
        chunks::{ChunkCoords, ChunkGenTask, DataChunk, GridData, MapDataProducer},
        constants::{DEFAULT_CHUNK_DIMENSION_TILES, TILE_SIZE_IN_UNITS},
        units::TilesCount,
    },
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
    transform::components::Transform,
};

use crate::Player;

/// The central resource for managing a chunked map of type T using double buffering.
#[derive(Resource)]
pub struct DataMapDoubleBuffered<P: MapDataProducer> {
    /// The front buffer, for reading data (e.g., by rendering systems).
    /// This represents the complete, consistent state of the world for the current frame.
    /// All read operations should use this buffer.
    read_buffer: HashMap<ChunkCoords, DataChunk<P::GridType>>,

    /// The back buffer, for writing new data (e.g., from generation tasks).
    /// This is prepared during the frame to become the next `read_buffer`.
    /// All write operations and generation results are directed here.
    write_buffer: HashMap<ChunkCoords, DataChunk<P::GridType>>,

    pub requested_chunks: HashSet<ChunkCoords>,
    // Maps chunk coords to the entity holding the generation task
    pub pending_tasks: HashMap<ChunkCoords, Entity>,
    pub write_queue: HashMap<Point, P::Item>, // Writes to uncreated/unloaded cells
    pub producer: P,
    pub chunk_dimension_tiles: TilesCount,
    pub chunk_size_units: f32, // Derived from chunk_dimension_tiles and TILE_SIZE_IN_UNITS
    pub render_distance_chunks: usize, // Used by the map manager system to determine loading radius
}

impl<P: MapDataProducer> DataMapDoubleBuffered<P> {
    pub fn new(
        producer: P,
        chunk_dimension_tiles: TilesCount,
        render_distance_chunks: usize,
    ) -> Self {
        let chunk_size_units = chunk_dimension_tiles as f32 * TILE_SIZE_IN_UNITS;
        Self {
            read_buffer: HashMap::new(),
            write_buffer: HashMap::new(),
            requested_chunks: HashSet::new(),
            pending_tasks: HashMap::new(),
            write_queue: HashMap::new(),
            producer,
            chunk_dimension_tiles,
            chunk_size_units,
            render_distance_chunks,
        }
    }

    /// Swaps the read and write buffers.
    /// This is typically called once per frame/tick (e.g., in `PostUpdate`) after all
    /// generation and updates for the frame have been applied to the `write_buffer`.
    /// This makes the newly prepared state available for reading in the next frame.
    pub fn swap_buffers(&mut self) {
        mem::swap(&mut self.read_buffer, &mut self.write_buffer);
    }

    // --- Public API for Game Logic ---

    /// Provides immutable access to the read buffer for chunks. For reading state.
    pub fn read_chunks(&self) -> &HashMap<ChunkCoords, DataChunk<P::GridType>> {
        &self.read_buffer
    }

    /// Provides mutable access to the write buffer for chunks. For modifying state.
    pub fn write_chunks(&mut self) -> &mut HashMap<ChunkCoords, DataChunk<P::GridType>> {
        &mut self.write_buffer
    }

    /// Gets the data at a specific world tile Point from the **read buffer**.
    /// If the chunk is not loaded, it spawns a chunk generation request and returns a default value.
    pub fn get(&mut self, point: Point) -> P::Item {
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return queued_value;
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.read_buffer.get(&chunk_coords) {
            let local_x = (point.x.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            let local_y = (point.y.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            chunk
                .grid
                .get_item(local_x, local_y)
                .copied()
                .unwrap_or_else(|| self.producer.default_value())
        } else {
            self.requested_chunks.insert(chunk_coords);
            self.producer.default_value()
        }
    }

    /// Gets the data at a specific floating-point world position from the **read buffer**.
    pub fn get_rounded(&mut self, world_pos: Vec2) -> P::Item {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.get(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Attempts to get data from the **read buffer**. Returns `None` if not loaded.
    /// Spawns a chunk generation request if the chunk is not loaded.
    pub fn get_option(&mut self, point: Point) -> Option<P::Item> {
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return Some(queued_value);
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.read_buffer.get(&chunk_coords) {
            let local_x = (point.x.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            let local_y = (point.y.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            chunk.grid.get_item(local_x, local_y).copied()
        } else {
            self.requested_chunks.insert(chunk_coords);
            None
        }
    }

    /// Attempts to get data from a rounded world position from the **read buffer**.
    pub fn get_rounded_option(&mut self, world_pos: Vec2) -> Option<P::Item> {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.get_option(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Reads data from the **read buffer** without spawning generation requests.
    pub fn read(&self, point: Point) -> Option<P::Item> {
        if let Some(&queued_value) = self.write_queue.get(&point) {
            return Some(queued_value);
        }

        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        self.read_buffer.get(&chunk_coords).and_then(|chunk| {
            let local_x = (point.x.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            let local_y = (point.y.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            chunk.grid.get_item(local_x, local_y).copied()
        })
    }

    /// Reads data from a rounded world position from the **read buffer** without generation requests.
    pub fn read_rounded(&self, world_pos: Vec2) -> Option<P::Item> {
        let tile_x = (world_pos.x / TILE_SIZE_IN_UNITS).round() as isize;
        let tile_y = (world_pos.y / TILE_SIZE_IN_UNITS).round() as isize;
        self.read(Point {
            x: tile_x,
            y: tile_y,
        })
    }

    /// Writes data to a specific world tile Point, targeting the **write buffer**.
    /// If the chunk is loaded in the write buffer, it's modified immediately.
    /// If not, the write is queued for when the chunk is generated.
    pub fn write(&mut self, point: Point, value: P::Item) {
        let chunk_coords = ChunkCoords::from_point(point, self.chunk_dimension_tiles);
        if let Some(chunk) = self.write_buffer.get_mut(&chunk_coords) {
            let local_x = (point.x.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            let local_y = (point.y.rem_euclid(self.chunk_dimension_tiles as isize)) as TilesCount;
            chunk.grid.set_item(local_x, local_y, value);
            self.write_queue.remove(&point);
        } else {
            self.write_queue.insert(point, value);
            self.requested_chunks.insert(chunk_coords);
        }
    }

    /// Initializes chunks in a radius, spawning generation requests.
    pub fn init(&mut self, manhattan_distance_tiles: usize) {
        let center_chunk = ChunkCoords { x: 0, y: 0 };
        let chunk_manhattan_distance =
            (manhattan_distance_tiles as f32 / self.chunk_dimension_tiles as f32).ceil() as isize;

        for x_offset in -chunk_manhattan_distance..=chunk_manhattan_distance {
            for y_offset in -chunk_manhattan_distance..=chunk_manhattan_distance {
                let current_chunk_coords = ChunkCoords {
                    x: center_chunk.x + x_offset,
                    y: center_chunk.y + y_offset,
                };
                if !self.read_buffer.contains_key(&current_chunk_coords)
                    && !self.write_buffer.contains_key(&current_chunk_coords)
                    && !self.pending_tasks.contains_key(&current_chunk_coords)
                {
                    self.requested_chunks.insert(current_chunk_coords);
                }
            }
        }
    }
}

// System to manage loading/unloading based on a focus point (e.g., player/camera)
// This system now prepares the WRITE BUFFER for the next frame.
pub fn data_map_db_load_unload_system<P: MapDataProducer>(
    player_query: Query<&Transform, With<MapRevealActor>>,
    mut data_map: ResMut<DataMapDoubleBuffered<P>>,
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

        // Unload chunks from the write buffer that are no longer required.
        // After a swap, the write_buffer contains the old read_buffer's state.
        // We trim it down to only what's needed for the next frame.
        data_map
            .write_buffer
            .retain(|coords, _| required_chunks_set.contains(coords));

        // Request new chunks that are required but not yet in the write buffer or pending.
        for coords in required_chunks_set.iter() {
            if !data_map.write_buffer.contains_key(coords)
                && !data_map.pending_tasks.contains_key(coords)
                && !data_map.requested_chunks.contains(coords)
            {
                data_map.requested_chunks.insert(*coords);
            }
        }
    }
}

pub fn data_map_db_load_unload_system_for_player<P: MapDataProducer>(
    player_query: Query<&Transform, With<Player>>,
    mut data_map: ResMut<DataMapDoubleBuffered<P>>,
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

        // Unload chunks from the write buffer that are no longer required.
        data_map
            .write_buffer
            .retain(|coords, _| required_chunks_set.contains(coords));

        // Request new chunks.
        for coords in required_chunks_set.iter() {
            if !data_map.write_buffer.contains_key(coords)
                && !data_map.pending_tasks.contains_key(coords)
                && !data_map.requested_chunks.contains(coords)
            {
                data_map.requested_chunks.insert(*coords);
            }
        }
    }
}

// System to spawn background tasks for requested chunks
pub fn data_map_spawn_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut data_map: ResMut<DataMapDoubleBuffered<P>>,
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

// System to process completed background tasks, inserting results into the WRITE BUFFER
pub fn data_map_process_completed_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut query: Query<(Entity, &ChunkCoords, &mut ChunkGenTask<P::GridType>)>,
    mut data_map: ResMut<DataMapDoubleBuffered<P>>,
) {
    for (task_entity, coords, mut gen_task) in query.iter_mut() {
        if let Some(mut generated_chunk) = future::block_on(future::poll_once(&mut gen_task.0)) {
            // Apply any writes from the queue to this newly generated chunk
            let chunk_bottom_left_tile =
                coords.to_bottom_left_tile_point(data_map.chunk_dimension_tiles);
            let chunk_dimension_tiles = data_map.chunk_dimension_tiles;

            data_map.write_queue.retain(|&point, value| {
                if point.x >= chunk_bottom_left_tile.x
                    && point.x < chunk_bottom_left_tile.x + chunk_dimension_tiles as isize
                    && point.y >= chunk_bottom_left_tile.y
                    && point.y < chunk_bottom_left_tile.y + chunk_dimension_tiles as isize
                {
                    let local_x =
                        (point.x.rem_euclid(chunk_dimension_tiles as isize)) as TilesCount;
                    let local_y =
                        (point.y.rem_euclid(chunk_dimension_tiles as isize)) as TilesCount;
                    generated_chunk.grid.set_item(local_x, local_y, *value);
                    false // Remove from queue
                } else {
                    true // Keep in queue
                }
            });

            // Insert the completed chunk into the write buffer
            data_map.write_buffer.insert(*coords, generated_chunk);

            commands.entity(task_entity).despawn();
            data_map.pending_tasks.remove(coords);
        }
    }
}

/// System that calls swap_buffers at the end of the frame.
pub fn swap_map_buffers_system<P: MapDataProducer>(mut data_map: ResMut<DataMapDoubleBuffered<P>>) {
    data_map.swap_buffers();
}

pub fn insert_chunked_double_buffered_plugin<P>(
    app: &mut bevy::prelude::App,
    producer: P,
    manhattan_distance_tiles_init: TilesCount,
) -> &mut bevy::prelude::App
where
    P: MapDataProducer + Send + Sync + Clone + 'static,
    <P as MapDataProducer>::GridType: Send + Sync,
    <P as MapDataProducer>::Item: Send + Copy + Default + Sync,
{
    app.add_systems(Startup, move |mut map: ResMut<DataMapDoubleBuffered<P>>| {
        map.init(manhattan_distance_tiles_init)
    });
    app.insert_resource(DataMapDoubleBuffered::<P>::new(
        producer,
        DEFAULT_CHUNK_DIMENSION_TILES,
        DEFAULT_RENDER_DISTANCE_CHUNKS,
    ))
    .add_systems(
        Update,
        (
            // Note: you might want to order these systems explicitly
            data_map_db_load_unload_system::<P>,
            data_map_spawn_tasks_system::<P>,
            data_map_process_completed_tasks_system::<P>,
        ),
    )
    // Add the system to swap buffers at the end of the update cycle.
    .add_systems(PostUpdate, swap_map_buffers_system::<P>);

    app
}

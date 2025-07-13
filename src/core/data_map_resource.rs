use bevy::{
    ecs::{entity::Entity, resource::Resource}, log::info, math::Vec2, platform::collections::{HashMap, HashSet}
};

use crate::core::{
    basics::{ChunkCoords, DataChunk, GridData, Point, TILE_SIZE_IN_UNITS},
    data_map_producer::MapDataProducer,
};

/// The central resource for managing a chunked map of type T.
#[derive(Resource)]
pub struct DataMap<P: MapDataProducer> {
    pub loaded_chunks: HashMap<ChunkCoords, DataChunk<P::GridType>>,
    pub requested_chunks: HashSet<ChunkCoords>,
    // Maps chunk coords to the entity holding the generation task
    pub pending_tasks: HashMap<ChunkCoords, Entity>,
    pub write_queue: HashMap<Point, P::Item>, // Writes to uncreated/unloaded cells
    pub producer: P,
    pub chunk_dimension_tiles: u32,
    pub chunk_size_units: f32, // Derived from chunk_dimension_tiles and TILE_SIZE_IN_UNITS
    pub render_distance_chunks: usize, // Used by the map manager system to determine loading radius
}

impl<P: MapDataProducer> DataMap<P> {
    pub fn new(producer: P, chunk_dimension_tiles: u32, render_distance_chunks: usize) -> Self {
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
                .get_item(local_x as u32, local_y as u32)
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
            chunk.grid.get_item(local_x as u32, local_y as u32).copied()
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
            chunk.grid.get_item(local_x as u32, local_y as u32).copied()
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
            chunk.grid.set_item(local_x as u32, local_y as u32, value);
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
        info!(
            "DataMap<{}> init requested chunks up to Manhattan distance {} (chunks: {})",
            std::any::type_name::<P::Item>(),
            manhattan_distance_tiles,
            chunk_manhattan_distance
        );
    }
}

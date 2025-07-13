use bevy::{
    ecs::system::Command,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Add, Sub},
}; // For polling tasks

// --- Constants (adjust as needed) ---
pub const TILE_SIZE_IN_UNITS: f32 = 16.0; // World units per tile
pub const DEFAULT_CHUNK_DIMENSION_TILES: u32 = 16; // 16x16 tiles per chunk
pub const DEFAULT_RENDER_DISTANCE_CHUNKS: usize = 3; // Load 3 chunks out from focus
pub const GAME_WORLD_CENTER_THRESHOLD: f32 = 10.0; // Distance from 0,0 where passability becomes 0

// --- Coordinate Structs ---

/// Absolute world tile coordinates.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
pub struct Point {
    pub x: isize,
    pub y: isize,
}

impl Add for Point {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

/// Absolute chunk coordinates.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
pub struct ChunkCoords {
    pub x: isize,
    pub y: isize,
}

impl ChunkCoords {
    /// Converts a world tile `Point` to `ChunkCoords`.
    pub fn from_point(point: Point, chunk_dimension_tiles: u32) -> Self {
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
    pub fn to_bottom_left_tile_point(&self, chunk_dimension_tiles: u32) -> Point {
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

// --- Common Grid Data (reusing FlatGrid from previous answer) ---
pub trait GridData: Send + Sync + 'static + Debug + Clone {
    type Item: Copy + Debug + Default; // Default trait required for new()
    fn dimension(&self) -> u32;
    fn get_item(&self, x: u32, y: u32) -> Option<&Self::Item>;
    fn get_item_mut(&mut self, x: u32, y: u32) -> Option<&mut Self::Item>;
    fn set_item(&mut self, x: u32, y: u32, item: Self::Item) -> bool;
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
}

#[derive(Debug, Clone)]
pub struct FlatGrid<T>
where
    T: Copy + Debug + Send + Sync + 'static + Default,
{
    data: Vec<T>,
    dimension: u32,
}

impl<T> FlatGrid<T>
where
    T: Copy + Debug + Send + Sync + 'static + Default,
{
    pub fn new(dimension: u32, default_value: T) -> Self {
        let num_elements = (dimension * dimension) as usize;
        FlatGrid {
            data: vec![default_value; num_elements],
            dimension,
        }
    }

    fn calculate_index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.dimension && y < self.dimension {
            Some((y * self.dimension + x) as usize)
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

    fn dimension(&self) -> u32 {
        self.dimension
    }

    fn get_item(&self, x: u32, y: u32) -> Option<&Self::Item> {
        self.calculate_index(x, y).map(|idx| &self.data[idx])
    }

    fn get_item_mut(&mut self, x: u32, y: u32) -> Option<&mut Self::Item> {
        self.calculate_index(x, y).map(|idx| &mut self.data[idx])
    }

    fn set_item(&mut self, x: u32, y: u32, item: Self::Item) -> bool {
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

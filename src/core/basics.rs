use bevy::prelude::*;
use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Add, Sub},
};

use crate::core::units::Units; // For polling tasks

pub const DEFAULT_RENDER_DISTANCE_CHUNKS: usize = 3; // Load 3 chunks out from focus
pub const GAME_WORLD_CENTER_THRESHOLD: f32 = 10.0; // Distance from 0,0 where passability becomes 0

// --- Coordinate Structs ---

/// Absolute world tile coordinates.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
pub struct Point {
    pub x: Units,
    pub y: Units,
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

impl Point {
    /// Creates a new `Point` from any integer-like types for x and y.
    ///
    /// # Arguments
    /// * `x` - The x-coordinate, convertible to `Units` (isize).
    /// * `y` - The y-coordinate, convertible to `Units` (isize).
    ///
    /// # Returns
    /// A new `Point` instance.
    pub fn new<T, U>(x: T, y: U) -> Self
    where
        T: TryInto<Units>,
        T::Error: std::fmt::Debug, // Required for unwrap, or handle error explicitly
        U: TryInto<Units>,
        U::Error: std::fmt::Debug, // Required for unwrap, or handle error explicitly
    {
        // Using unwrap here for simplicity, but in production code, you might
        // want to handle the error from `try_into` more gracefully,
        // especially if converting from a very large `usize` that could
        // overflow `isize`.
        Self {
            x: x.try_into().unwrap(),
            y: y.try_into().unwrap(),
        }
    }

    /// Converts world coordinates to tile coordinates
    /// 
    /// # Arguments
    /// * `world_pos` - World position as Vec2
    /// * `tile_size` - Size of each tile (square tiles)
    /// 
    /// # Returns
    /// Point representing the tile coordinates
    /// 
    /// # Example
    /// ```
    /// let world_pos = Vec2::new(150.0, 75.0);
    /// let tile_size = 32;
    /// let tile_point = Point::from_world_pos(world_pos, tile_size);
    /// // tile_point would be Point { x: 4, y: 2 }
    /// ```
    pub fn from_world_pos(world_pos: Vec2, tile_size: Units) -> Self {
        let tile_size_f32 = tile_size as f32;
        Self {
            x: (world_pos.x / tile_size_f32).floor() as Units,
            y: (world_pos.y / tile_size_f32).floor() as Units,
        }
    }

    /// Converts tile coordinates back to world coordinates (center of tile)
    /// 
    /// # Arguments
    /// * `tile_size` - Size of each tile
    /// 
    /// # Returns
    /// Vec2 representing the world position at the center of the tile
    pub fn to_world_pos(&self, tile_size: Units) -> Vec2 {
        let tile_size_f32 = tile_size as f32;
        Vec2::new(
            self.x as f32 * tile_size_f32 + tile_size_f32 * 0.5,
            self.y as f32 * tile_size_f32 + tile_size_f32 * 0.5,
        )
    }

    /// Converts tile coordinates to world coordinates (top-left corner of tile)
    /// 
    /// # Arguments
    /// * `tile_size` - Size of each tile
    /// 
    /// # Returns
    /// Vec2 representing the world position at the top-left corner of the tile
    pub fn to_world_pos_corner(&self, tile_size: Units) -> Vec2 {
        let tile_size_f32 = tile_size as f32;
        Vec2::new(
            self.x as f32 * tile_size_f32,
            self.y as f32 * tile_size_f32,
        )
    }
}

impl<T, U> From<(T, U)> for Point
where
    T: TryInto<Units>,
    T::Error: std::fmt::Debug,
    U: TryInto<Units>,
    U::Error: std::fmt::Debug,
{
    fn from(coords: (T, U)) -> Self {
        Point::new(coords.0, coords.1)
    }
}

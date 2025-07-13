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
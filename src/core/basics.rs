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

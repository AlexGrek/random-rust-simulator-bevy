use bevy::reflect::{Reflect, TypePath};

use crate::core::basics::{ChunkCoords, DataChunk, GridData};

// Trait that defines how a specific type of map data is generated.
// --- MapDataProducer Trait (slightly modified) ---
pub trait MapDataProducer: Send + Sync + 'static + Clone {
    type Item: Copy + Default + Send + Sync;
    type GridType: GridData<Item = Self::Item> + Send + Sync;

    /// Returns the default value for an ungenerated tile.
    fn default_value(&self) -> Self::Item;

    /// Generates a chunk of data for the given coordinates.
    /// Returns the DataChunk asset.
    fn generate_chunk(&self, coords: ChunkCoords, dimension_tiles: u32) -> DataChunk<Self::GridType>;
}

use bevy::utils::default;

use crate::{
    core::{
        chunks::{ChunkCoords, DataChunk, FlatGrid, MapDataProducer},
        units::TilesCount,
    },
    game::render::light_sim::lights::UndirectedLightEmitter,
};

// Passability DataProducer
#[derive(Default, Clone)]
pub struct LightsMapProducer;

// Passability
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LightEmitterCell {
    pub undirected_lights: Option<UndirectedLightEmitter>,
}

impl MapDataProducer for LightsMapProducer {
    type Item = LightEmitterCell;
    type GridType = FlatGrid<LightEmitterCell>;

    fn default_value(&self) -> Self::Item {
        LightEmitterCell::default()
    }

    fn generate_chunk(
        &self,
        coords: ChunkCoords,
        dimension_tiles: TilesCount,
    ) -> DataChunk<Self::GridType> {
        let grid = FlatGrid::new(dimension_tiles, self.default_value());
        DataChunk { grid }
    }
}

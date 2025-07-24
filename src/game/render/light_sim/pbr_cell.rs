use bevy::log::info;

use crate::core::{
    chunks::{ChunkCoords, DataChunk, FlatGrid, GridData, MapDataProducer},
    units::TilesCount,
};

#[derive(Default, Clone)]
pub struct PbrCellProducer;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PbrCell {
    pub absorbtion: f32,
}

impl MapDataProducer for PbrCellProducer {
    type Item = PbrCell;
    type GridType = FlatGrid<PbrCell>;

    fn default_value(&self) -> Self::Item {
        PbrCell::default()
    }

    fn generate_chunk(
        &self,
        coords: ChunkCoords,
        dimension_tiles: TilesCount,
    ) -> DataChunk<Self::GridType> {
        let mut grid = FlatGrid::new(dimension_tiles, self.default_value());

        for y in 0..dimension_tiles {
            for x in 0..dimension_tiles {
                let world_tile_x = coords.x * dimension_tiles as isize + x as isize;
                let world_tile_y = coords.y * dimension_tiles as isize + y as isize;

                let dist_from_center =
                    ((world_tile_x as f32).powi(2) + (world_tile_y as f32).powi(2)).sqrt();

                if dist_from_center < 1.0 {
                    info!("Created a pbr cell at {x}, {y} (from center: {dist_from_center})");
                    grid.set_item(x, y, PbrCell::default());
                }
            }
        }

        DataChunk { grid }
    }
}

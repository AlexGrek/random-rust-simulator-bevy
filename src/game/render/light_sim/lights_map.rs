use bevy::{
    color::{palettes::css, ColorToPacked}, log::info
};

use crate::{
    core::{
        chunks::{ChunkCoords, DataChunk, FlatGrid, GridData, MapDataProducer},
        units::TilesCount,
    },
    game::render::light_sim::lights::UndirectedLightEmitter,
};

// Passability DataProducer
#[derive(Default, Clone)]
pub struct LightsMapProducer;

// Passability
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
        let mut grid = FlatGrid::new(dimension_tiles, self.default_value());

        for y in 0..dimension_tiles {
            for x in 0..dimension_tiles {
                let world_tile_x = coords.x * dimension_tiles as isize + x as isize;
                let world_tile_y = coords.y * dimension_tiles as isize + y as isize;

                let dist_from_center =
                    ((world_tile_x as f32).powi(2) + (world_tile_y as f32).powi(2)).sqrt();

                let color = css::LIMEGREEN;

                if dist_from_center < 3.0 {
                    info!("Created a light cell at {x}, {y} (from center: {dist_from_center})");
                    grid.set_item(
                        x,
                        y,
                        LightEmitterCell {
                            undirected_lights: Some(UndirectedLightEmitter {
                                props: color.into(),
                            }),
                        },
                    );
                }
                
            }
        }

        DataChunk { grid }
    }
}

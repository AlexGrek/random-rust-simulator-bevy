use bevy::log::info;

use crate::core::{
    chunks::{ChunkCoords, DataChunk, FlatGrid, GridData, MapDataProducer},
    units::TilesCount,
};

#[derive(Default, Clone)]
pub struct PbrCellProducer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PbrCell {
    pub transparent: bool,
    pub absorbtion: f32,
    pub reflection: f32,
    pub scattering: f32,
}

impl Default for PbrCell {
    fn default() -> Self {
        Self {
            transparent: true,
            absorbtion: 0.9,
            reflection: 0.0,
            scattering: 0.1,
        }
    }
}

impl PbrCell {
    /// Represents a semi-transparent glass material.
    pub const SEMI_TRANSPARENT_GLASS: PbrCell = PbrCell {
        transparent: true,
        absorbtion: 0.1,  // Low absorption for transparency
        reflection: 0.2,  // Some reflection, like glass
        scattering: 0.05, // Minimal scattering for clarity
    };

    /// Represents a solid, opaque wall.
    pub const SOLID_WALL: PbrCell = PbrCell {
        transparent: false,
        absorbtion: 1.0,  // High absorption, no light passes
        reflection: 0.1,  // Some diffuse reflection
        scattering: 0.1,  // Some scattering on the surface
    };

    /// Represents a highly reflective wall (mirror).
    pub const REFLECTIVE_WALL: PbrCell = PbrCell {
        transparent: false,
        absorbtion: 0.0,  // No absorption
        reflection: 0.95, // Very high reflection
        scattering: 0.0,  // No scattering
    };

    /// Represents a medium fog material.
    pub const MEDIUM_FOG: PbrCell = PbrCell {
        transparent: true,
        absorbtion: 0.2,  // Moderate absorption
        reflection: 0.1,  // Some light reflects off particles
        scattering: 0.7,  // High scattering for foggy effect
    };

    /// Represents a heavy fog material.
    pub const HEAVY_FOG: PbrCell = PbrCell {
        transparent: true,
        absorbtion: 0.4,  // Higher absorption
        reflection: 0.05, // Less reflection than medium fog, more diffuse
        scattering: 0.9,  // Very high scattering for dense fog
    };
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

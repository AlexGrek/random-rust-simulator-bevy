use bevy::prelude::*;

use crate::{
    core::{basics::Point, chunks::DataMap, constants::TILE_SIZE_IN_UNITS_UNITS},
    game::render::light_sim::{
        directions::Direction,
        lighting::{LIGHTING_OVERLAY_TILES, LightOverlayTextureHandle, OverlayImage},
        lights_map::{LightEmitterCell, LightsMapProducer},
    },
};

#[derive(Resource)]
pub struct LightingBuffers {
    pub read: [Vec<Vec<[i32; 3]>>; 8],
    pub write: [Vec<Vec<[i32; 3]>>; 8],
}

impl LightingBuffers {
    /// Initializes LightingBuffers with correctly sized zeroed buffers.
    pub fn init(write_size: usize) -> Self {
        let blank_tile = || vec![vec![[0; 3]; write_size]; write_size];
        Self {
            read: std::array::from_fn(|_| blank_tile()),
            write: std::array::from_fn(|_| blank_tile()),
        }
    }

    /// Swaps read/write and zeroes out the write buffer in-place.
    #[inline(always)]
    pub fn swap_buffers_clear_write(&mut self) {
        std::mem::swap(&mut self.read, &mut self.write);

        for dir_buf in self.write.iter_mut() {
            for row in dir_buf.iter_mut() {
                for px in row.iter_mut() {
                    *px = [0; 3];
                }
            }
        }
    }
}

impl Default for LightingBuffers {
    fn default() -> Self {
        Self {
            read: std::array::from_fn(|_| vec![]),
            write: std::array::from_fn(|_| vec![]),
        }
    }
}

pub fn run_lights_simulation(
    mut light_texture: ResMut<LightOverlayTextureHandle>,
    lightsources: Res<DataMap<LightsMapProducer>>,
    texture_world_position: Query<&Transform, With<OverlayImage>>,
    mut buffer: Local<LightingBuffers>,
) {
    // initialize buffers (if not yet initialized)
    if buffer.read[0].is_empty() {
        for dir in Direction::ALL {
            buffer.read[dir as usize] =
                vec![
                    vec![std::array::from_fn(|_| 0); LIGHTING_OVERLAY_TILES];
                    LIGHTING_OVERLAY_TILES
                ];
        }
        for dir in Direction::ALL {
            buffer.write[dir as usize] =
                vec![
                    vec![std::array::from_fn(|_| 0); LIGHTING_OVERLAY_TILES];
                    LIGHTING_OVERLAY_TILES
                ];
        }
    }
    // read lights map and produce a buffer for all directions
    let world_position_opt = texture_world_position.single();
    if let Ok(texture_position) = world_position_opt {
        // Convert overlay texture center world position (Vec3) to tile coordinates
        let center_world_x = texture_position.translation.x as isize;
        let center_world_y = texture_position.translation.y as isize;

        let center_tile = Point {
            x: center_world_x / TILE_SIZE_IN_UNITS_UNITS,
            y: center_world_y / TILE_SIZE_IN_UNITS_UNITS,
        };

        // Calculate top-left tile of the overlay area
        let half_tiles = (LIGHTING_OVERLAY_TILES / 2) as isize;
        let top_left = Point {
            x: center_tile.x - half_tiles,
            y: center_tile.y - half_tiles,
        };

        // Fill the 2D overlay buffer
        for x in 0..LIGHTING_OVERLAY_TILES {
            for y in 0..LIGHTING_OVERLAY_TILES {
                let tile_position = Point {
                    x: top_left.x + x as isize,
                    y: top_left.y + y as isize,
                };

                let center_tile_lightdata_cell =
                    lightsources.read(tile_position).unwrap_or_default();

                let center_tile_lightdata = center_tile_lightdata_cell.undirected_lights;

                if let Some(light) = center_tile_lightdata {
                    for dir in Direction::ALL {
                        buffer.write[dir as usize][x][y] = light.props.color;
                    }
                }
            }
        }

        // dummy simulation logic - do nothing for now

        // render result
        light_texture
    }
}

use bevy::{
    color::{color_difference::EuclideanDistance, palettes::css},
    prelude::*,
};
use rand::{Rng, random_bool};

use bevy::prelude::*;

use crate::{
    core::{basics::Point, chunks::DataMap, constants::TILE_SIZE_IN_UNITS_UNITS},
    game::render::{
        blending::MultiplyBlendMaterial,
        light_sim::{
            color_utils,
            directions::Direction,
            lighting::{
                LightOverlayMaterialHandle, LightOverlayTextureHandle, OverlayImage, LIGHTING_OVERLAY_TILES
            },
            lights_map::LightsMapProducer, pbr_cell::PbrCellProducer,
        },
    },
};

#[derive(Resource)]
pub struct LightingBuffers {
    pub read: [Vec<Vec<[f32; 3]>>; 8],
    pub write: [Vec<Vec<[f32; 3]>>; 8],
    pub initialized: bool,
}

impl LightingBuffers {
    /// Initializes LightingBuffers with correctly sized zeroed buffers.
    pub fn init(&mut self, write_size: usize) {
        let blank_tile = || vec![vec![[0.0; 3]; write_size]; write_size];
        self.read = std::array::from_fn(|_| blank_tile());
        self.write = std::array::from_fn(|_| blank_tile());
        self.initialized = true;
        info!("Double buffer for light sim initialized");
    }

    /// Swaps read/write and zeroes out the write buffer in-place.
    #[inline(always)]
    pub fn swap_buffers_clear_write(&mut self) {
        std::mem::swap(&mut self.read, &mut self.write);

        for dir_buf in self.write.iter_mut() {
            for row in dir_buf.iter_mut() {
                for px in row.iter_mut() {
                    *px = [0.0; 3];
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
            initialized: false,
        }
    }
}

pub fn run_lights_simulation(
    light_texture_handle: Res<LightOverlayTextureHandle>,
    mut images: ResMut<Assets<Image>>,
    lightsources: Res<DataMap<LightsMapProducer>>,
    pbr_cells: Res<DataMap<PbrCellProducer>>,
    texture_world_position: Query<&Transform, With<OverlayImage>>,
    mut buffer: Local<LightingBuffers>,
    light_material_handle: Res<LightOverlayMaterialHandle>,
    mut materials: ResMut<Assets<MultiplyBlendMaterial>>,
) {
    // initialize buffers (if not yet initialized)
    if !buffer.initialized {
        buffer.init(LIGHTING_OVERLAY_TILES);
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
                    // info!("Light detected: {x},{y}: {:?}", light.props.color);
                    for dir in Direction::ALL {
                        buffer.write[dir as usize][x][y] = light.props.color;
                    }
                }
            }
        }
        buffer.swap_buffers_clear_write();

        // dummy simulation logic - do nothing for now
        simulate_directions(&mut buffer);

        // render result
        let image = images
            .get_mut(&light_texture_handle.0)
            .expect("Image not found");
        for x in 0..LIGHTING_OVERLAY_TILES {
            for y in 0..LIGHTING_OVERLAY_TILES {
                let total_px = LIGHTING_OVERLAY_TILES as usize;
                let color = Color::from(Srgba::from_f32_array_no_alpha(
                    buffer.read[Direction::E as usize][x][y],
                ));
                let x_coords = x;
                let y_coords: isize = total_px as isize - (y as isize + 1);
                image
                    .set_color_at(x_coords as u32, y_coords as u32, color)
                    .unwrap();
            }
        }
    }
    materials.get_mut(&light_material_handle.0);
}

fn simulate_directions(
    mut buffer: &mut LightingBuffers
) {

}

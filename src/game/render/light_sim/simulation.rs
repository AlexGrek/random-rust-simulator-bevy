use bevy::{
    color::{color_difference::EuclideanDistance, palettes::css},
    prelude::*,
};
use rand::{Rng, random_bool};

use bevy::prelude::*;

use crate::{
    core::{basics::Point, chunks::DataMap, constants::TILE_SIZE_IN_UNITS_UNITS},
    game::render::light_sim::{
        color_utils,
        directions::Direction,
        lighting::{
            LIGHTING_OVERLAY_TILES, LightOverlayTextureHandle, OVERLAY_IMAGE_SIZE, OverlayImage,
        },
        lights_map::LightsMapProducer,
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

/// Every fixed update tick, draw one more pixel to make a spiral pattern
pub(crate) fn draw(
    my_handle: Res<LightOverlayTextureHandle>,
    mut images: ResMut<Assets<Image>>,
    // Used to keep track of where we are
    mut i: Local<u32>,
    mut draw_color: Local<Color>,
) {
    if *i == 0 {
        // Generate a random color on first run.
        let mut rng = rand::rng();
        let color = Color::srgb(
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
        );
        *draw_color = color;
    }

    // Get the image from Bevy's asset storage.
    let image = images.get_mut(&my_handle.0).expect("Image not found");

    // Compute the position of the pixel to draw.

    let center = Vec2::new(16 as f32 / 2.0, 16 as f32 / 2.0);
    let max_radius = 16.min(16) as f32 / 2.0;
    let rot_speed = 0.0123;
    let period = 0.12345;

    let r = ops::sin(*i as f32 * period) * max_radius;
    let xy = Vec2::from_angle(*i as f32 * rot_speed) * r + center;
    let (x, y) = (xy.x as u32, xy.y as u32);

    // Get the old color of that pixel.
    let old_color = image.get_color_at(x, y).unwrap();

    // If the old color is our current color, change our drawing color.
    let tolerance = 1.0 / 255.0;
    if old_color.distance(&draw_color) <= tolerance {
        let mut rng = rand::rng();
        let color = Color::srgb(
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
        );
        *draw_color = color;
    }

    // Set the new color, but keep old alpha value from image.
    image
        .set_color_at(x, y, draw_color.with_alpha(old_color.alpha()))
        .unwrap();

    *i += 1;
}

/// Draws one random pixel with a random color at each update
pub fn run_fucking_simulation(
    light_texture_handle: Res<LightOverlayTextureHandle>,
    mut images: ResMut<Assets<Image>>,
) {
    // Get mutable reference to the image
    if let Some(image) = images.get_mut(&light_texture_handle.0) {
        let width = image.size().x;
        let height = image.size().y;

        // Generate random coordinates and color
        let mut rng = rand::rng();
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);

        let color = Color::srgb(
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
            rng.gen_range(0.0..=1.0),
        );

        // Set the pixel
        image.set_color_at(x, y, color).unwrap();
    }
}

pub fn run_lights_simulation(
    light_texture_handle: Res<LightOverlayTextureHandle>,
    mut images: ResMut<Assets<Image>>,
    lightsources: Res<DataMap<LightsMapProducer>>,
    texture_world_position: Query<&Transform, With<OverlayImage>>,
    mut buffer: Local<LightingBuffers>,
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

                // if let Some(light) = center_tile_lightdata {
                //     info!("Light detected: {x},{y}: {:?}", light.props.color);
                //     for dir in Direction::ALL {
                //         buffer.write[dir as usize][x][y] = light.props.color;
                //     }
                // }
            }
        }
        buffer.swap_buffers_clear_write();

        // dummy simulation logic - do nothing for now

        // render result
        let image = images
            .get_mut(&light_texture_handle.0)
            .expect("Image not found");
        for x in 0..LIGHTING_OVERLAY_TILES {
            for y in 0..LIGHTING_OVERLAY_TILES {
                let texture_y_px = y * TILE_SIZE_IN_UNITS_UNITS as usize;
                let total_px = OVERLAY_IMAGE_SIZE as usize;
                if buffer.read[Direction::E as usize][x][y][0] > 0.0 {
                    let my_x = x * TILE_SIZE_IN_UNITS_UNITS as usize;
                    let my_y =
                        total_px - (texture_y_px as isize + TILE_SIZE_IN_UNITS_UNITS) as usize;
                    let srgba =
                        Srgba::from_f32_array_no_alpha(buffer.read[Direction::E as usize][x][y]);
                    // info!(
                    //     "We detected something: {:?} at {my_x} {my_y}: {:?}; reference color: {:?}",
                    //     buffer.read[Direction::E as usize][x][y],
                    //     Color::from(srgba),
                    //     Color::from(css::LIMEGREEN)
                    // );
                }
                // if buffer.read[Direction::E as usize][x][y][0] == 0 {
                //     utils::draw_rect_on_image(
                //         &mut image,
                //         x * TILE_SIZE_IN_UNITS_UNITS as usize,
                //         total_px - (texture_y_px as isize + TILE_SIZE_IN_UNITS_UNITS) as usize,
                //         TILE_SIZE_IN_UNITS_UNITS as usize,
                //         TILE_SIZE_IN_UNITS_UNITS as usize,
                //         css::DARK_GREEN.to_u8_array(),
                //     );
                // }
                // utils::draw_rect_on_image(
                //     &mut image,
                //     x * TILE_SIZE_IN_UNITS_UNITS as usize,
                //     total_px - (texture_y_px as isize + TILE_SIZE_IN_UNITS_UNITS) as usize,
                //     TILE_SIZE_IN_UNITS_UNITS as usize,
                //     TILE_SIZE_IN_UNITS_UNITS as usize,
                //     color_utils::convert_color(buffer.read[Direction::E as usize][x][y]),
                // );
                let color = if random_bool(0.5) {
                    Color::from(Srgba::from_f32_array_no_alpha(
                        buffer.read[Direction::E as usize][x][y],
                    ))
                } else {
                    Color::from(css::LIMEGREEN)
                };
                image.set_color_at(x as u32, y as u32, color).unwrap();
            }
        }
    }
    let image = images
        .get(&light_texture_handle.0)
        .expect("Image not found")
        .clone();
    images.insert(&light_texture_handle.0, image);
}

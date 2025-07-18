use bevy::{
    asset::{Handle, RenderAssetUsages},
    color::{self, ColorToPacked, palettes::css},
    ecs::{
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    image::Image,
    math::Vec3Swizzles,
    platform::collections::HashSet,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Sprite,
    transform::components::Transform,
};

use crate::{
    core::{
        basics::Point,
        chunks::{ChunkCoords, DataMap},
        constants::TILE_SIZE_IN_UNITS_UNITS,
        units::{TilesCount, Units},
    },
    game::{
        MapRevealActor,
        render::utils,
        world::passability::{self, PassabilityProducer},
    },
};

const IMAGE_WIDTH_PX: u32 = 64;
const IMAGE_HEIGHT_PX: u32 = IMAGE_WIDTH_PX;
const IMAGE_WIDTH_TILES: TilesCount = IMAGE_WIDTH_PX as usize / TILE_SIZE_IN_UNITS_UNITS as usize;
const IMAGE_HEIGHT_TILES: TilesCount = IMAGE_HEIGHT_PX as usize / TILE_SIZE_IN_UNITS_UNITS as usize;
const MAP_RENDER_DISTANCE: isize = 2;

#[derive(Resource)]
struct BackgroundHypertile(ChunkCoords, Handle<Image>);

#[derive(Resource)]
pub struct BackgroundHypertileTracker {
    pub spawned: HashSet<ChunkCoords>,
    pub requested: HashSet<ChunkCoords>,
}

impl BackgroundHypertileTracker {
    pub fn require(&mut self, coords: ChunkCoords) {
        if self.spawned.contains(&coords) || self.requested.contains(&coords) {
            return; // already exists or already requested
        }
        self.requested.insert(coords);
    }

    pub fn mark_all_requests_as_completed(&mut self) {
        self.spawned.extend(self.requested.drain());
    }
}

pub fn background_load_unload_system(
    player_query: Query<&Transform, With<MapRevealActor>>,
    mut tracker: ResMut<BackgroundHypertileTracker>,
) {
    for player_transform in player_query.as_readonly().iter() {
        let focus_world_pos = player_transform.translation.xy();
        let current_focus_chunk_coords =
            ChunkCoords::from_world_pos(focus_world_pos, IMAGE_HEIGHT_PX as f32);

        for dx in -(MAP_RENDER_DISTANCE)..=(MAP_RENDER_DISTANCE) {
            for dy in -(MAP_RENDER_DISTANCE)..=(MAP_RENDER_DISTANCE) {
                tracker.require(ChunkCoords {
                    x: current_focus_chunk_coords.x + dx,
                    y: current_focus_chunk_coords.y + dy,
                });
            }
        }
    }
}

pub fn background_load_required_chunks_system(
    mut passability_map: ResMut<DataMap<PassabilityProducer>>,
    mut tracker: ResMut<BackgroundHypertileTracker>,
    mut commands: Commands,
    mut images: ResMut<bevy::asset::Assets<Image>>,
) {
    if tracker.requested.is_empty() {
        return;
    }
    let mut deferred = HashSet::new();
    let process_requested_chunk = |requested_chunk: &ChunkCoords| {
        let real_coords_bottom_left = requested_chunk.to_world_pos(IMAGE_WIDTH_PX as f32); // our chunk size is image size
        let tiles_coords_of_a_chunk = requested_chunk.to_bottom_left_tile_point(IMAGE_WIDTH_TILES);
        let passability = passability_map.get_rounded_option(real_coords_bottom_left);
        if passability.is_none() {
            deferred.insert(requested_chunk.clone());
            return;
        }
        let color = if passability.is_none() {
            css::BEIGE.to_u8_array()
        } else {
            if passability.unwrap().0 > 0 {
                css::BLUE_VIOLET.to_u8_array()
            } else {
                css::STEEL_BLUE.to_u8_array()
            }
        };

        let mut image = Image::new_fill(
            // 2D image of size
            Extent3d {
                width: IMAGE_WIDTH_PX,
                height: IMAGE_HEIGHT_PX,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            // Initialize it with a beige color
            &(color),
            // Use the same encoding as the color we set
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );

        let tiles = IMAGE_WIDTH_TILES;
        for i in 0..tiles {
            for j in 0..tiles {
                let p = passability_map.get_option(Point {
                    x: i as isize + tiles_coords_of_a_chunk.x,
                    y: j as isize + tiles_coords_of_a_chunk.y,
                });
                let color_exact = if p.is_none() {
                    css::BEIGE.to_u8_array()
                } else {
                    if p.unwrap().0 > 10 {
                        (i as u8 * 16, j as u8 * 16, 128, 255 as u8).into()
                    } else {
                        (i as u8 * 16, j as u8 * 16, 42, 255 as u8).into()
                    }
                };
                let texture_y_px = j * TILE_SIZE_IN_UNITS_UNITS as usize;
                let total_px = IMAGE_WIDTH_PX as usize;
                utils::draw_rect_on_image(
                    &mut image,
                    i * TILE_SIZE_IN_UNITS_UNITS as usize,
                    total_px - (texture_y_px as isize + TILE_SIZE_IN_UNITS_UNITS) as usize,
                    TILE_SIZE_IN_UNITS_UNITS as usize,
                    TILE_SIZE_IN_UNITS_UNITS as usize,
                    color_exact,
                );
            }
        }

        let handle = images.add(image);
        let offset: f32 = (IMAGE_WIDTH_TILES as f32 / 2.0 - 0.5) * TILE_SIZE_IN_UNITS_UNITS as f32;
        let x = requested_chunk.x as f32 * IMAGE_HEIGHT_PX as f32 + offset;
        let y = requested_chunk.y as f32 * IMAGE_WIDTH_PX as f32 + offset;
        commands.spawn((
            Sprite::from_image(handle),
            Transform::from_xyz(x, y, 0.0),
        ));
    };

    let _: Vec<_> = tracker
        .requested
        .iter()
        .map(process_requested_chunk)
        .collect();
    tracker.mark_all_requests_as_completed();
    tracker.requested = deferred;
}

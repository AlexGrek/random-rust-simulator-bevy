use bevy::{
    ecs::{
        query::With,
        system::{Local, Query, ResMut},
    },
    log::info,
    transform::components::Transform,
};

use crate::{
    core::{
        basics::{Point, GAME_WORLD_CENTER_THRESHOLD},
        chunks::{ChunkCoords, DataChunk, DataMap, FlatGrid, GridData, MapDataProducer},
        constants::TILE_SIZE_IN_UNITS, units::TilesCount,
    },
    game::Player,
};

// Passability
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Passability(pub u8);

impl Passability {
    pub const IMPASSABLE: Passability = Passability(0);
    pub const FREE: Passability = Passability(255);
}

// Passability DataProducer
#[derive(Default, Clone)]
pub struct PassabilityProducer;

impl MapDataProducer for PassabilityProducer {
    type Item = Passability;
    type GridType = FlatGrid<Passability>;

    fn default_value(&self) -> Self::Item {
        Passability::IMPASSABLE
    }

    fn generate_chunk(
        &self,
        coords: ChunkCoords,
        dimension_tiles: TilesCount,
    ) -> DataChunk<Self::GridType> {
        let mut grid = FlatGrid::new(dimension_tiles, Passability::FREE);
        let chunk_center_x =
            coords.x as f32 * dimension_tiles as f32 + dimension_tiles as f32 / 2.0;
        let chunk_center_y =
            coords.y as f32 * dimension_tiles as f32 + dimension_tiles as f32 / 2.0;

        for y in 0..dimension_tiles {
            for x in 0..dimension_tiles {
                let world_tile_x = coords.x * dimension_tiles as isize + x as isize;
                let world_tile_y = coords.y * dimension_tiles as isize + y as isize;

                let dist_from_center =
                    ((world_tile_x as f32).powi(2) + (world_tile_y as f32).powi(2)).sqrt();

                // Make tiles impassable further from center
                let mut passability = Passability::FREE;
                if dist_from_center > GAME_WORLD_CENTER_THRESHOLD {
                    let falloff = (dist_from_center - GAME_WORLD_CENTER_THRESHOLD) / 500.0;
                    info!("Falloff: {}", &falloff);
                    passability = Passability((255.0 - (falloff * 255.0).min(255.0)) as u8);
                    if passability.0 < 250 {
                        passability = Passability(0);
                    }
                }

                // // Example: Add a small "river" (impassable)
                // if world_tile_y > 100 && world_tile_y < 105 {
                //     passability = Passability::IMPASSABLE;
                // }

                grid.set_item(x, y, passability);
            }
        }

        DataChunk { grid }
    }
}

pub fn check_player_passability(
    player_query: Query<&Transform, With<Player>>,
    mut passability_map: ResMut<DataMap<PassabilityProducer>>, // Needs mut to make requests
    mut last_checked_point: Local<Option<Point>>,
) {
    let player_transform = player_query.single().unwrap();
    let player_tile_point = Point {
        x: (player_transform.translation.x / TILE_SIZE_IN_UNITS).round() as isize,
        y: (player_transform.translation.y / TILE_SIZE_IN_UNITS).round() as isize,
    };

    if last_checked_point.map_or(true, |p| p != player_tile_point) {
        *last_checked_point = Some(player_tile_point);
        let passability = passability_map.get(player_tile_point); // This will request the chunk if not loaded
        info!(
            "Player at tile {:?} has passability: {:?}",
            player_tile_point, passability
        );

        // Example: Try writing
        if passability.0 == Passability::FREE.0 {
            // passability_map.write(player_tile_point + Point{x:1, y:0}, Passability::IMPASSABLE);
            // info!("Queued write to make tile {:?} impassable", player_tile_point + Point{x:1, y:0});
        }
    }
}

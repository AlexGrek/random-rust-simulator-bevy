use std::sync::Arc;

use bevy::prelude::Vec3Swizzles;
use bevy::{
    ecs::{
        entity::Entity,
        query::With,
        system::{Commands, Query, ResMut},
    },
    log::info,
    platform::collections::HashSet,
    tasks::AsyncComputeTaskPool,
    transform::components::Transform,
};
use futures_lite::future;

use crate::{
    Player,
    core::{
        basics::{ChunkCoords, ChunkGenTask, GridData},
        data_map_producer::MapDataProducer,
        data_map_resource::DataMap,
    },
};

// System to manage loading/unloading based on a focus point (e.g., player/camera)
pub fn data_map_load_unload_system<P: MapDataProducer>(
    player_query: Query<&Transform, With<Player>>,
    mut data_map: ResMut<DataMap<P>>,
) {
    let player_transform = player_query.single().unwrap();
    let focus_world_pos = player_transform.translation.xy();
    let current_focus_chunk_coords =
        ChunkCoords::from_world_pos(focus_world_pos, data_map.chunk_size_units);

    let mut required_chunks_set: HashSet<ChunkCoords> = HashSet::new();

    for dx in
        -(data_map.render_distance_chunks as isize)..=(data_map.render_distance_chunks as isize)
    {
        for dy in
            -(data_map.render_distance_chunks as isize)..=(data_map.render_distance_chunks as isize)
        {
            required_chunks_set.insert(ChunkCoords {
                x: current_focus_chunk_coords.x + dx,
                y: current_focus_chunk_coords.y + dy,
            });
        }
    }

    // Unload chunks that are no longer required
    data_map
        .loaded_chunks
        .retain(|coords, _| required_chunks_set.contains(coords));

    // Request new chunks
    for coords in required_chunks_set.iter() {
        if !data_map.loaded_chunks.contains_key(coords) &&
           !data_map.pending_tasks.contains_key(coords) && // Don't request if already pending
           !data_map.requested_chunks.contains(coords)
        // Don't request if already in queue
        {
            data_map.requested_chunks.insert(*coords);
        }
    }
}

// System to spawn background tasks for requested chunks
pub fn data_map_spawn_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut data_map: ResMut<DataMap<P>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    let mut new_pending_tasks = Vec::new(); // Collect tasks to add to pending_tasks map

    let producer = Arc::new(data_map.producer.clone());

    for coords in data_map.requested_chunks.iter() {
        if !data_map.pending_tasks.contains_key(coords) {
            // Double check for safety
            let chunk_dimension = data_map.chunk_dimension_tiles;
            let current_coords = *coords;
            let pr = producer.clone();

            let task = thread_pool
                .spawn(async move { pr.generate_chunk(current_coords, chunk_dimension) });

            let task_entity = commands
                .spawn((
                    current_coords, // Attach coords for easy lookup by completion system
                    ChunkGenTask(task),
                ))
                .id();

            new_pending_tasks.push((current_coords, task_entity));
            info!(
                "Spawned DataMap<{}> gen task for chunk: {:?}",
                std::any::type_name::<P::Item>(),
                current_coords
            );
        }
    }

    // Add new pending tasks to the map and clear requested chunks
    for (coords, entity) in new_pending_tasks {
        data_map.pending_tasks.insert(coords, entity);
    }
    data_map.requested_chunks.clear();
}

// System to process completed background tasks
pub fn data_map_process_completed_tasks_system<P: MapDataProducer>(
    mut commands: Commands,
    mut query: Query<(Entity, &ChunkCoords, &mut ChunkGenTask<P::GridType>)>,
    mut data_map: ResMut<DataMap<P>>,
) {
    let mut completed_chunks = Vec::new();

    for (task_entity, coords, mut gen_task) in query.iter_mut() {
        if let Some(generated_chunk) = future::block_on(future::poll_once(&mut gen_task.0)) {
            completed_chunks.push((*coords, generated_chunk));
            commands.entity(task_entity).despawn(); // Remove the temporary task entity
            data_map.pending_tasks.remove(coords); // Remove from pending map
        }
    }

    // Apply completed chunks and pending writes
    for (coords, mut chunk) in completed_chunks {
        info!(
            "DataMap<{}> chunk {:?} generated.",
            std::any::type_name::<P::Item>(),
            coords
        );
        // Apply any writes from the queue to this newly generated chunk
        let chunk_bottom_left_tile =
            coords.to_bottom_left_tile_point(data_map.chunk_dimension_tiles);

        // Use a temporary Vec to collect points to remove from write_queue
        let mut points_to_remove = Vec::new();

        let chunk_dimension_tiles = data_map.chunk_dimension_tiles;

        // Iterate over the write_queue and identify items to apply
        for (&point, &value) in data_map.write_queue.iter() {
            if point.x >= chunk_bottom_left_tile.x
                && point.x < chunk_bottom_left_tile.x + chunk_dimension_tiles as isize
                && point.y >= chunk_bottom_left_tile.y
                && point.y < chunk_bottom_left_tile.y + chunk_dimension_tiles as isize
            {
                // This write belongs to the newly generated chunk
                let local_x = (point.x % chunk_dimension_tiles as isize
                    + chunk_dimension_tiles as isize)
                    % chunk_dimension_tiles as isize;
                let local_y = (point.y % chunk_dimension_tiles as isize
                    + chunk_dimension_tiles as isize)
                    % chunk_dimension_tiles as isize;
                chunk.grid.set_item(local_x as u32, local_y as u32, value);
                points_to_remove.push(point); // Mark for removal
            }
        }

        // Now, remove the marked points from the write_queue outside the iteration
        for point in points_to_remove {
            data_map.write_queue.remove(&point);
        }

        data_map.loaded_chunks.insert(coords, chunk);
    }
}

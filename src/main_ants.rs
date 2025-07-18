// main.rs

use bevy::{
    color::palettes::css::{ANTIQUE_WHITE, FUCHSIA, GOLD, LIGHT_CYAN, LIMEGREEN},
    prelude::*,
    sprite::MeshMaterial2d,
    time::common_conditions::on_timer,
};
use rand::{Rng, rng};
use std::time::Duration;

// --- Constants ---
const SCREEN_WIDTH: f32 = 1024.0;
const SCREEN_HEIGHT: f32 = 1024.0;

const MAP_WIDTH: usize = 256;
const MAP_HEIGHT: usize = 256;
const PIXEL_SCALE: f32 = 4.0; // Each map pixel represents a 4x4 screen pixel area

const ANT_COUNT: usize = 150;
const FOOD_COUNT: usize = 20;

const ANT_SPEED: f32 = 60.0;
const ANT_ROTATION_SPEED: f32 = 3.5;
const ANT_SIGHT_DISTANCE: f32 = 20.0;
const ANT_SIGHT_ANGLE: f32 = std::f32::consts::FRAC_PI_4; // 45 degrees

const PHEROMONE_SPRAY_COOLDOWN: f32 = 0.1;
const ESCAPE_COOLDOWN: f32 = 5.0;
const ESCAPE_DURATION: f32 = 2.0;

// Pheromone intensities and decay rates
const HOME_PHEROMONE_INTENSITY: f32 = 1.0;
const PATH_PHEROMONE_INTENSITY: f32 = 0.8;
const SUCCESS_PHEROMONE_INTENSITY: f32 = 1.0;
const DANGER_PHEROMONE_INTENSITY: f32 = 1.0;

const PHEROMONE_DECAY_RATE: f32 = 0.1; // Amount per second

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ant Simulation".into(),
                resolution: (SCREEN_WIDTH, SCREEN_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_camera, setup_simulation))
        .add_systems(
            Update,
            (
                ant_decision_system,
                ant_movement_system,
                ant_state_transition_system,
                pheromone_spraying_system,
                pheromone_decay_system.run_if(on_timer(Duration::from_secs_f32(0.1))),
                update_pheromone_visualization,
                queen_emits_pheromones.run_if(on_timer(Duration::from_secs_f32(0.5))),
            ),
        )
        .run();
}

// --- Components ---

#[derive(Component)]
struct Ant;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum AntState {
    Seeking,
    CarryingFood,
    Escaping,
}

#[derive(Component)]
struct Target(Vec2);

#[derive(Component)]
struct Direction(Vec2);

#[derive(Component)]
struct PheromoneSprayCooldown(Timer);

#[derive(Component)]
struct EscapeState {
    timer: Timer,
    cooldown: Timer,
}

#[derive(Component)]
struct Food;

#[derive(Component)]
struct Queen;

#[derive(Component)]
struct CarriedBy(Entity);

#[derive(Component)]
struct PheromoneVisual; // Marker component for the visualization grid sprites

// --- Resources ---

#[derive(Resource)]
struct Terrain {
    // true = passable, false = impassable
    grid: Vec<Vec<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PheromoneType {
    Home,
    Path,
    Success,
    Danger,
}

#[derive(Resource)]
struct PheromoneGrids {
    home: Vec<Vec<f32>>,
    path: Vec<Vec<f32>>,
    success: Vec<Vec<f32>>,
    danger: Vec<Vec<f32>>,
}

impl PheromoneGrids {
    fn new(width: usize, height: usize) -> Self {
        Self {
            home: vec![vec![0.0; height]; width],
            path: vec![vec![0.0; height]; width],
            success: vec![vec![0.0; height]; width],
            danger: vec![vec![0.0; height]; width],
        }
    }

    fn get_grid_mut(&mut self, p_type: PheromoneType) -> &mut Vec<Vec<f32>> {
        match p_type {
            PheromoneType::Home => &mut self.home,
            PheromoneType::Path => &mut self.path,
            PheromoneType::Success => &mut self.success,
            PheromoneType::Danger => &mut self.danger,
        }
    }

    fn get_grid(&self, p_type: PheromoneType) -> &Vec<Vec<f32>> {
        match p_type {
            PheromoneType::Home => &self.home,
            PheromoneType::Path => &self.path,
            PheromoneType::Success => &self.success,
            PheromoneType::Danger => &self.danger,
        }
    }

    // Add pheromone at a specific world position
    fn add(&mut self, world_pos: Vec2, p_type: PheromoneType, value: f32) {
        if let Some((x, y)) = world_to_grid_pos(world_pos) {
            let grid = self.get_grid_mut(p_type);
            grid[x][y] = (grid[x][y] + value).min(1.0);
        }
    }

    // Sense pheromones in a cone in front of the ant
    fn sense_in_cone(
        &self,
        pos: Vec2,
        dir: Vec2,
        angle: f32,
        distance: f32,
    ) -> (Option<(PheromoneType, Vec2)>, Option<(PheromoneType, Vec2)>) {
        let mut best_target: Option<(PheromoneType, Vec2, f32)> = None;
        let mut danger_target: Option<(PheromoneType, Vec2, f32)> = None;

        for p_type in [
            PheromoneType::Success,
            PheromoneType::Path,
            PheromoneType::Home,
            PheromoneType::Danger,
        ] {
            let grid = self.get_grid(p_type);
            // Check a few points in the cone
            for i in 0..=10 {
                let check_angle = angle * ((i as f32 / 10.0) - 0.5); // from -angle/2 to +angle/2
                let check_dir = Quat::from_rotation_z(check_angle)
                    .mul_vec3(dir.extend(0.0))
                    .truncate();
                let check_pos = pos + check_dir * distance;

                if let Some((gx, gy)) = world_to_grid_pos(check_pos) {
                    let intensity = grid[gx][gy];
                    if intensity > 0.01 {
                        let target_pos = grid_to_world_pos(gx, gy);
                        if p_type == PheromoneType::Danger {
                            if danger_target.is_none() || intensity > danger_target.unwrap().2 {
                                danger_target = Some((p_type, target_pos, intensity));
                            }
                        } else {
                            // Prioritize success pheromones
                            let priority = if p_type == PheromoneType::Success {
                                2.0
                            } else {
                                1.0
                            };
                            let weighted_intensity = intensity * priority;

                            if best_target.is_none() || weighted_intensity > best_target.unwrap().2
                            {
                                best_target = Some((p_type, target_pos, weighted_intensity));
                            }
                        }
                    }
                }
            }
        }
        (
            best_target.map(|(t, p, _)| (t, p)),
            danger_target.map(|(t, p, _)| (t, p)),
        )
    }
}

// --- Systems ---

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn setup_simulation(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // --- Terrain and Pheromones ---
    let mut terrain_grid = vec![vec![true; MAP_HEIGHT]; MAP_WIDTH];
    // Example: Create a simple impassable border
    for x in 0..MAP_WIDTH {
        terrain_grid[x][0] = false;
        terrain_grid[x][MAP_HEIGHT - 1] = false;
    }
    for y in 0..MAP_HEIGHT {
        terrain_grid[0][y] = false;
        terrain_grid[MAP_WIDTH - 1][y] = false;
    }

    let mut rng = rand::rng();

    // Draw terrain for visualization
    for _ in 0..FOOD_COUNT {
        let food_pos = Vec2::new(
            (rng.random::<f32>() - 0.5) * (SCREEN_WIDTH * 0.8),
            (rng.random::<f32>() - 0.5) * (SCREEN_HEIGHT * 0.8),
        );
        // Ensure food doesn't spawn too close to the nest
        if food_pos.length() > 150.0 {
            commands.spawn((
                Food,
                // Use Mesh2dBundle for a 2D mesh, and assign the material directly
                Mesh2d(meshes.add(Circle::new(5.0))), // Circle directly from bevy::math
                MeshMaterial2d(materials.add(Color::from(LIMEGREEN))), // Explicitly create and add ColorMaterial
                Transform::from_translation(food_pos.extend(1.0)),
            ));
        }
    }

    commands.insert_resource(Terrain { grid: terrain_grid });
    commands.insert_resource(PheromoneGrids::new(MAP_WIDTH, MAP_HEIGHT));

    // Draw pheromone visualization grid
    // commands.spawn(SpatialBundle::default()).with_children(|parent| {
    //     for x in 0..MAP_WIDTH {
    //         for y in 0..MAP_HEIGHT {
    //             parent.spawn((
    //                 SpriteBundle {
    //                     sprite: Sprite {
    //                         color: Color::NONE, // Initially invisible
    //                         custom_size: Some(Vec2::splat(PIXEL_SCALE)),
    //                         ..default()
    //                     },
    //                     transform: Transform::from_translation(grid_to_world_pos(x, y).extend(0.1)), // Slightly above terrain
    //                     ..default()
    //                 },
    //                 PheromoneVisual,
    //             ));
    //         }
    //     }
    // });

    // --- Queen (Nest) ---
    let queen_pos = Vec2::new(0.0, 0.0);
    commands.spawn((
        Queen,
        Mesh2d(meshes.add(Circle::new(15.0)).into()),
        MeshMaterial2d(materials.add(Color::from(GOLD))),
        Transform::from_translation(queen_pos.extend(1.0)),
    ));

    // --- Ants ---
    for _ in 0..ANT_COUNT {
        let start_pos = queen_pos
            + Vec2::new(
                rand::random::<f32>() * 40.0 - 20.0,
                rand::random::<f32>() * 40.0 - 20.0,
            );
        let start_dir = Vec2::new(
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
        )
        .normalize_or_zero();
        commands.spawn((
            Ant,
            AntState::Seeking,
            Direction(start_dir),
            Target(start_pos + start_dir * 50.0),
            PheromoneSprayCooldown(Timer::from_seconds(
                PHEROMONE_SPRAY_COOLDOWN,
                TimerMode::Repeating,
            )),
            EscapeState {
                timer: Timer::from_seconds(ESCAPE_DURATION, TimerMode::Once),
                cooldown: Timer::from_seconds(ESCAPE_COOLDOWN, TimerMode::Once),
            },
            Mesh2d(
                meshes
                    .add(Triangle2d::new(
                        Vec2::new(0.0, 6.0),
                        Vec2::new(-4.0, -6.0),
                        Vec2::new(4.0, -6.0),
                    ))
                    .into(),
            ),
            MeshMaterial2d(materials.add(Color::from(ANTIQUE_WHITE))),
            Transform::from_translation(start_pos.extend(2.0)),
        ));
    }

    // --- Food ---
    for _ in 0..FOOD_COUNT {
        let food_pos = Vec2::new(
            (rand::random::<f32>() - 0.5) * (SCREEN_WIDTH * 0.8),
            (rand::random::<f32>() - 0.5) * (SCREEN_HEIGHT * 0.8),
        );
        // Ensure food doesn't spawn too close to the nest
        if food_pos.length() > 150.0 {
            commands.spawn((
                Food,
                Mesh2d(meshes.add(Circle::new(5.0)).into()),
                MeshMaterial2d(materials.add(Color::from(LIMEGREEN))),
                Transform::from_translation(food_pos.extend(1.0)),
            ));
        }
    }
}

// The main "brain" of the ant. It decides what to do based on its state and senses.
fn ant_decision_system(
    mut ant_query: Query<(&mut Target, &Transform, &Direction, &AntState, &EscapeState)>,
    pheromones: Res<PheromoneGrids>,
    queen_query: Query<&Transform, (With<Queen>, Without<Ant>)>,
) {
    let queen_transform = queen_query.single();
    let queen_pos = queen_transform.unwrap().translation.truncate();

    for (mut target, transform, direction, state, escape_state) in ant_query.iter_mut() {
        // Don't make new decisions while escaping
        if *state == AntState::Escaping && !escape_state.timer.finished() {
            continue;
        }

        let pos = transform.translation.truncate();
        let dir = direction.0;

        // --- Sense the environment ---
        let (best_pheromone, danger_pheromone) =
            pheromones.sense_in_cone(pos, dir, ANT_SIGHT_ANGLE, ANT_SIGHT_DISTANCE);

        // --- State-based Decision Making ---
        // High-priority: React to danger if not on cooldown
        if let Some((_, danger_pos)) = danger_pheromone {
            if escape_state.cooldown.finished() {
                // New target is away from the danger
                let away_dir = (pos - danger_pos).normalize_or_zero();
                *target = Target(pos + away_dir * 50.0);
                // State transition will be handled in another system
                continue; // Skip other logic for this frame
            }
        }

        match state {
            AntState::Seeking => {
                if let Some((p_type, p_pos)) = best_pheromone {
                    // Seeking ants prioritize success pheromones
                    if p_type == PheromoneType::Success {
                        *target = Target(p_pos);
                    }
                    // If no success pheromones, they might follow path pheromones away from home
                    else if p_type == PheromoneType::Path {
                        // Only follow path if it leads away from the nest
                        if (p_pos - queen_pos).length_squared() > (pos - queen_pos).length_squared()
                        {
                            *target = Target(p_pos);
                        }
                    }
                } else {
                    // No pheromones found, check if current target is reached
                    if (pos - target.0).length_squared() < 10.0 * 10.0 {
                        // Pick a new random point in a small cone
                        let angle_offset = (rand::random::<f32>() - 0.5) * ANT_SIGHT_ANGLE;
                        let new_dir = Quat::from_rotation_z(angle_offset)
                            .mul_vec3(dir.extend(0.0))
                            .truncate();
                        *target = Target(pos + new_dir * 100.0);
                    }
                }
            }
            AntState::CarryingFood => {
                if let Some((p_type, p_pos)) = best_pheromone {
                    // When carrying food, prioritize home pheromones
                    if p_type == PheromoneType::Home {
                        *target = Target(p_pos);
                    } else {
                        // If no home pheromones are sensed, head directly to the queen
                        *target = Target(queen_pos);
                    }
                } else {
                    *target = Target(queen_pos);
                }
            }
            AntState::Escaping => {
                // Logic is handled by the danger check above and state transition system
            }
        }
    }
}

// Handles state changes based on environmental interactions and timers.
fn ant_state_transition_system(
    mut commands: Commands,
    mut ant_query: Query<
        (
            Entity,
            &mut AntState,
            &mut EscapeState,
            &Transform,
            Option<&CarriedBy>,
        ),
        With<Ant>,
    >,
    food_query: Query<(Entity, &Transform), (With<Food>, Without<CarriedBy>)>,
    queen_query: Query<&Transform, With<Queen>>,
    pheromones: Res<PheromoneGrids>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let queen_transform = queen_query.single();
    let queen_pos = queen_transform.unwrap().translation.truncate();

    for (ant_entity, mut state, mut escape_state, ant_transform, carried_by) in ant_query.iter_mut()
    {
        let ant_pos = ant_transform.translation.truncate();

        // --- Handle Escape State Timer and Cooldown ---
        if *state == AntState::Escaping {
            escape_state.timer.tick(time.delta());
            if escape_state.timer.just_finished() {
                *state = AntState::Seeking; // Revert to seeking after escaping
                escape_state.cooldown.reset(); // Start cooldown
                // *material = materials.add(Color::from(ANTIQUE_WHITE));s
            }
        } else {
            // Cooldown ticks even when not escaping
            escape_state.cooldown.tick(time.delta());
        }

        // --- Check for Danger ---
        // This check is separate to allow immediate reaction
        let ant_dir = (ant_transform.rotation * Vec3::Y).truncate();
        if let Some((_, _)) = pheromones
            .sense_in_cone(ant_pos, ant_dir, ANT_SIGHT_ANGLE, ANT_SIGHT_DISTANCE)
            .1
        {
            if escape_state.cooldown.finished() && *state != AntState::Escaping {
                *state = AntState::Escaping;
                escape_state.timer.reset();
                if let Some(carried_comp) = carried_by {
                    commands.entity(carried_comp.0).remove::<CarriedBy>();
                }
                commands.entity(ant_entity).remove::<CarriedBy>();
                // *material = materials.add(Color::from(FUCHSIA));
            }
        }

        // --- State-specific transitions ---
        match *state {
            AntState::Seeking => {
                // Check for nearby food
                for (food_entity, food_transform) in food_query.iter() {
                    if (ant_pos - food_transform.translation.truncate()).length_squared()
                        < 10.0 * 10.0
                    {
                        *state = AntState::CarryingFood;
                        commands.entity(ant_entity).insert(CarriedBy(food_entity));
                        commands.entity(food_entity).insert(CarriedBy(ant_entity));
                        // *material = materials.add(Color::from(LIGHT_CYAN));
                        break; // Pick up one food item at a time
                    }
                }
            }
            AntState::CarryingFood => {
                // Check if near the queen to drop food
                if (ant_pos - queen_pos).length_squared() < 20.0 * 20.0 {
                    if let Some(carried_comp) = carried_by {
                        // Despawn food and remove carrying components
                        commands.entity(carried_comp.0).despawn();
                        commands.entity(ant_entity).remove::<CarriedBy>();
                        *state = AntState::Seeking;
                        // *material = materials.add(Color::from(ANTIQUE_WHITE));
                    }
                }
            }
            AntState::Escaping => { /* Handled above */ }
        }
    }
}

// Moves and rotates ants towards their target.
fn ant_movement_system(
    mut ant_query: Query<
        (
            &mut Transform,
            &mut Direction,
            &Target,
            &AntState,
            Option<&CarriedBy>,
        ),
        With<Ant>,
    >,
    mut food_transform_query: Query<&mut Transform, (With<Food>, Without<Ant>)>,
    time: Res<Time>,
    terrain: Res<Terrain>,
) {
    for (mut transform, mut direction, target, state, carried_by) in ant_query.iter_mut() {
        let pos = transform.translation.truncate();
        let to_target = (target.0 - pos).normalize_or_zero();

        if to_target.length_squared() > 0.0 {
            // --- Rotation ---
            let current_dir = direction.0;
            // Using atan2 for robust 2D rotation
            let target_angle = to_target.y.atan2(to_target.x);
            let current_angle = current_dir.y.atan2(current_dir.x);
            // Slerp the angle for smooth rotation
            let new_angle = slerp_angle(
                current_angle,
                target_angle,
                ANT_ROTATION_SPEED * time.delta_secs(),
            );

            transform.rotation = Quat::from_rotation_z(new_angle - std::f32::consts::FRAC_PI_2); // Adjust for sprite orientation
            direction.0 = Vec2::new(new_angle.cos(), new_angle.sin());

            // --- Translation ---
            let mut move_delta = direction.0 * ANT_SPEED * time.delta_secs();

            // --- Terrain Collision ---
            let next_pos = pos + move_delta;
            if let Some((gx, gy)) = world_to_grid_pos(next_pos) {
                if !terrain
                    .grid
                    .get(gx)
                    .and_then(|col| col.get(gy))
                    .unwrap_or(&false)
                {
                    // Hit a wall, stop movement
                    move_delta = Vec2::ZERO;
                }
            } else {
                // Outside map bounds
                move_delta = Vec2::ZERO;
            }

            transform.translation += move_delta.extend(0.0);

            // --- Move Carried Food ---
            if *state == AntState::CarryingFood {
                if let Some(carried_comp) = carried_by {
                    if let Ok(mut food_transform) = food_transform_query.get_mut(carried_comp.0) {
                        food_transform.translation =
                            transform.translation + direction.0.extend(0.0) * 10.0;
                    }
                }
            }
        }
    }
}

// Sprays pheromones based on the ant's current state.
fn pheromone_spraying_system(
    mut ant_query: Query<(&Transform, &AntState, &mut PheromoneSprayCooldown)>,
    mut pheromones: ResMut<PheromoneGrids>,
    time: Res<Time>,
) {
    for (transform, state, mut cooldown) in ant_query.iter_mut() {
        cooldown.0.tick(time.delta());
        if cooldown.0.just_finished() {
            let pos = transform.translation.truncate();
            match state {
                AntState::Seeking => {
                    pheromones.add(pos, PheromoneType::Path, PATH_PHEROMONE_INTENSITY);
                }
                AntState::CarryingFood => {
                    pheromones.add(pos, PheromoneType::Success, SUCCESS_PHEROMONE_INTENSITY);
                }
                AntState::Escaping => {
                    pheromones.add(pos, PheromoneType::Danger, DANGER_PHEROMONE_INTENSITY);
                }
            }
        }
    }
}

// The queen constantly emits home pheromones.
fn queen_emits_pheromones(
    queen_query: Query<&Transform, With<Queen>>,
    mut pheromones: ResMut<PheromoneGrids>,
) {
    let queen_transform = queen_query.single();
    pheromones.add(
        queen_transform.unwrap().translation.truncate(),
        PheromoneType::Home,
        HOME_PHEROMONE_INTENSITY,
    );
}

// Periodically reduces the intensity of all pheromones.
fn pheromone_decay_system(mut pheromones: ResMut<PheromoneGrids>) {
    for p_type in [
        PheromoneType::Home,
        PheromoneType::Path,
        PheromoneType::Success,
        PheromoneType::Danger,
    ] {
        let grid = pheromones.get_grid_mut(p_type);
        for x in 0..MAP_WIDTH {
            for y in 0..MAP_HEIGHT {
                grid[x][y] = (grid[x][y] - PHEROMONE_DECAY_RATE * 0.1).max(0.0);
            }
        }
    }
}

// Updates the color of the pheromone visualization grid sprites.
fn update_pheromone_visualization(
    mut query: Query<(&mut Sprite, &Transform), With<PheromoneVisual>>,
    pheromones: Res<PheromoneGrids>,
) {
    for (mut sprite, transform) in query.iter_mut() {
        if let Some((gx, gy)) = world_to_grid_pos(transform.translation.truncate()) {
            // Get intensities from all grids for the current position
            let home_intensity = pheromones.home[gx][gy];
            let path_intensity = pheromones.path[gx][gy];
            let success_intensity = pheromones.success[gx][gy];
            let danger_intensity = pheromones.danger[gx][gy];

            // Blend colors based on intensity
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a: f32 = 0.0;

            if home_intensity > 0.01 {
                r += 0.8 * home_intensity;
                g += 0.8 * home_intensity;
                b += 0.2 * home_intensity;
                a = a.max(home_intensity);
            }
            if path_intensity > 0.01 {
                r += 0.2 * path_intensity;
                g += 0.5 * path_intensity;
                b += 0.8 * path_intensity;
                a = a.max(path_intensity);
            }
            if success_intensity > 0.01 {
                r += 0.2 * success_intensity;
                g += 0.8 * success_intensity;
                b += 0.2 * success_intensity;
                a = a.max(success_intensity);
            }
            if danger_intensity > 0.01 {
                r += 0.9 * danger_intensity;
                g += 0.1 * danger_intensity;
                b += 0.1 * danger_intensity;
                a = a.max(danger_intensity);
            }

            if a > 0.0 {
                // Set the blended color, capping alpha for visibility
                sprite.color = Color::srgba(r.min(1.0), g.min(1.0), b.min(1.0), (a * 0.4).min(1.0));
            } else {
                sprite.color = Color::NONE;
            }
        } else {
            sprite.color = Color::NONE;
        }
    }
}

// --- Utility Functions ---

fn world_to_grid_pos(world_pos: Vec2) -> Option<(usize, usize)> {
    let offset_x = world_pos.x + (MAP_WIDTH as f32 * PIXEL_SCALE) / 2.0;
    let offset_y = world_pos.y + (MAP_HEIGHT as f32 * PIXEL_SCALE) / 2.0;

    let gx = (offset_x / PIXEL_SCALE).floor() as isize;
    let gy = (offset_y / PIXEL_SCALE).floor() as isize;

    if gx >= 0 && gx < MAP_WIDTH as isize && gy >= 0 && gy < MAP_HEIGHT as isize {
        Some((gx as usize, gy as usize))
    } else {
        None
    }
}

fn grid_to_world_pos(gx: usize, gy: usize) -> Vec2 {
    let world_x =
        (gx as f32 * PIXEL_SCALE) - (MAP_WIDTH as f32 * PIXEL_SCALE) / 2.0 + PIXEL_SCALE / 2.0;
    let world_y =
        (gy as f32 * PIXEL_SCALE) - (MAP_HEIGHT as f32 * PIXEL_SCALE) / 2.0 + PIXEL_SCALE / 2.0;
    Vec2::new(world_x, world_y)
}

// Spherically interpolates between two angles.
fn slerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let diff = (b - a + std::f32::consts::PI) % (2.0 * std::f32::consts::PI) - std::f32::consts::PI;
    a + diff * t
}

// ```toml
// # Cargo.toml

// [package]
// name = "bevy_ant_sim"
// version = "0.1.0"
// edition = "2021"

// # See more keys and their definitions at [https://doc.rust-lang.org/cargo/reference/manifest.html](https://doc.rust-lang.org/cargo/reference/manifest.html)

// [dependencies]
// bevy = "0.16.1"
// rand = "0.8.5"

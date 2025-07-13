use bevy::{ecs::{component::Component, query::With, system::{Query, Res}}, input::{keyboard::KeyCode, ButtonInput}, math::Vec3, transform::components::Transform};

pub mod world;


// --- Player Component for focus point ---
#[derive(Component)]
pub struct Player;

// --- Example Player movement system ---
pub fn player_movement(
    mut player_query: Query<&mut Transform, With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<bevy::time::Time>,
) {
    let transform = player_query.single_mut();
    let move_speed = 200.0; // units per second
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction != Vec3::ZERO {
        transform.unwrap().translation += direction.normalize() * move_speed * time.delta_secs();
    }
}

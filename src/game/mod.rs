use bevy::{
    ecs::{
        component::Component,
        query::With,
        system::{Query, Res},
    },
    input::{ButtonInput, keyboard::KeyCode},
    math::{Vec3, Vec3Swizzles},
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
};

use crate::{core::chunks::DataMap, game::{physix::PrevXY, world::passability::PassabilityProducer}, Pallete};

pub mod render;
pub mod world;
pub mod physix;

// --- Player Component for focus point ---
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct MapRevealActor;

// --- Example Player movement system ---
pub fn player_movement(
    mut player_query: Query<(&mut Transform, &mut PrevXY, &mut MeshMaterial2d<ColorMaterial>), With<Player>>,
    passability: Res<DataMap<PassabilityProducer>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<bevy::time::Time>,
    pallete: Res<Pallete>,
) {
    let (mut transform, mut prev, mut material) = player_query.single_mut().unwrap();
    let pass = passability.read_rounded(transform.translation.xy());
    if let Some(p) = pass {
        if p.0 < 200 {
            material.0 = pallete.colors.get("red").unwrap().clone();
        } else {
            material.0 = pallete.colors.get("limegreen").unwrap().clone();
        }
    }
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
        prev.0 = transform.translation.clone();
        transform.translation += direction.normalize() * move_speed * time.delta_secs();
    }
}

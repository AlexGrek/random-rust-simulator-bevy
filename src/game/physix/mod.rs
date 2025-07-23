use bevy::prelude::*;

use crate::{
    core::chunks::DataMap,
    game::world::passability::PassabilityProducer,
};

#[derive(Component, Default)]
pub struct PrevXY(pub Vec3);

pub fn bounce_back(
    q: Query<(&mut Transform, &PrevXY)>,
    passability: Res<DataMap<PassabilityProducer>>,
) {
    for (mut transform, prevxy) in q {
        let pass = passability.read_rounded(transform.translation.xy());
        if let Some(p) = pass {
            if p.0 < 10 {
                // impassable
                transform.translation = prevxy.0;
            } else {
                // passable
            }
        }
    }
}

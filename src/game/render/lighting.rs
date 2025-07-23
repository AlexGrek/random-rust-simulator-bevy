use bevy::{
    asset::{RenderAssetUsages, uuid},
    color::palettes::css,
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{
            AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d,
            RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError, TextureDimension,
            TextureFormat,
        },
    },
    sprite::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin},
};

use crate::{
    FollowCamera,
    core::{basics::Point, constants::TILE_SIZE_IN_UNITS_UNITS, units::Units},
    game::render::blending::{AdditiveMaterial, MultiplyBlendMaterial, ScreenBlendMaterial},
};

pub struct Lighting;

const OVERLAY_IMAGE_SIZE: Units = 120;

#[derive(Component)]
pub struct OverlayImage(Handle<Image>);

impl Plugin for Lighting {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<AdditiveMaterial>::default(),
            Material2dPlugin::<ScreenBlendMaterial>::default(),
            Material2dPlugin::<MultiplyBlendMaterial>::default(),
        ));
        // Register systems, resources, events, etc.
        app.add_systems(Update, overlay_texture_follow_camera);
        app.add_systems(Startup, setup_overlay);
    }
}

fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<bevy::asset::Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MultiplyBlendMaterial>>,
) {
    
    let color = css::GREY.to_u8_array();
    let mut image = Image::new_fill(
        // 2D image of size
        Extent3d {
            width: OVERLAY_IMAGE_SIZE as u32,
            height: OVERLAY_IMAGE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(color),
        // Use the same encoding as the color we set
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    // Additive Blend
    let additive_material = MultiplyBlendMaterial {
        texture: handle.clone(),
    };
    let mesh = meshes.add(Rectangle::new(
        OVERLAY_IMAGE_SIZE as f32,
        OVERLAY_IMAGE_SIZE as f32,
    ));
    commands.spawn((
        OverlayImage(handle.clone()),
        MeshMaterial2d(materials.add(additive_material)),
        Mesh2d(mesh),
        Transform::from_xyz(0.0, 0.0, 100000.0),
    ));
}

fn overlay_texture_follow_camera(
    mut overlay_image_q: Query<&mut Transform, (With<OverlayImage>, Without<FollowCamera>)>,
    camera_query: Query<&Transform, (With<FollowCamera>, Without<OverlayImage>)>,
) {
    if let Ok(camera_transform) = camera_query.single() {
        for mut transform in overlay_image_q.iter_mut() {
            let target_position =
                Point::from_world_pos(camera_transform.translation.xy(), TILE_SIZE_IN_UNITS_UNITS)
                    .to_world_pos(TILE_SIZE_IN_UNITS_UNITS);

            transform.translation = Vec3 {
                x: target_position.x,
                y: target_position.y,
                z: 0.0,
            }
        }
    }
}

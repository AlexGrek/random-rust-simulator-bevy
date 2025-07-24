use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    sprite::{Material2d, Material2dPlugin},
};

use crate::{
    core::{
        basics::Point,
        chunks::insert_chunked_plugin,
        constants::TILE_SIZE_IN_UNITS_UNITS,
        units::{TilesCount, Units},
    }, game::render::{
        blending::{AdditiveMaterial, MultiplyBlendMaterial, ScreenBlendMaterial},
        light_sim::{lights_map::LightsMapProducer, pbr_cell::PbrCellProducer, simulation},
    }, FollowCamera
};

pub struct Lighting;

pub const LIGHTING_OVERLAY_TILES: TilesCount = 32;
pub const OVERLAY_IMAGE_SIZE_SCALED: Units = LIGHTING_OVERLAY_TILES as isize * TILE_SIZE_IN_UNITS_UNITS;

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
        setup_directional_lights(app);
        app.add_systems(Startup, setup_overlay);
    }
}

fn setup_directional_lights(app: &mut App) {
    insert_chunked_plugin(app, LightsMapProducer::default(), 100);
    insert_chunked_plugin(app, PbrCellProducer::default(), 100);
    app.add_systems(PostUpdate, simulation::run_lights_simulation);
}

#[derive(Resource)]
pub struct LightOverlayTextureHandle(pub Handle<Image>);

#[derive(Resource)]
pub struct LightOverlayMaterialHandle(pub Handle<MultiplyBlendMaterial>);

fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MultiplyBlendMaterial>>,
) {
    let color = css::AQUAMARINE.to_u8_array();
    let size_unscaled = (OVERLAY_IMAGE_SIZE_SCALED / TILE_SIZE_IN_UNITS_UNITS) as u32;
    let image = Image::new_fill(
        // 2D image of size
        Extent3d {
            width: size_unscaled as u32,
            height: size_unscaled as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(color),
        // Use the same encoding as the color we set
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    let handle = images.add(image);

    // Additive Blend
    let additive_material = MultiplyBlendMaterial {
        texture: handle.clone(),
    };
    let mesh = meshes.add(Rectangle::new(
        OVERLAY_IMAGE_SIZE_SCALED as f32,
        OVERLAY_IMAGE_SIZE_SCALED as f32,
    ));
    let material_handle = materials.add(additive_material);
    commands.spawn((
        OverlayImage(handle.clone()),
        MeshMaterial2d(material_handle.clone()),
        Mesh2d(mesh),
        Transform::from_xyz(0.0, 0.0, 100000.0),
    ));
    commands.insert_resource(LightOverlayTextureHandle(handle));
    commands.insert_resource(LightOverlayMaterialHandle(material_handle));
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

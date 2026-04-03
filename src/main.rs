mod map;
mod wad;

use anyhow::anyhow;
use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension},
    window::WindowResolution,
    winit::WinitSettings,
};

use crate::{
    map::Map,
    wad::{WadFile, types::WadName},
};

#[derive(Resource)]
struct CommandLineArgs(Vec<String>);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    App::new()
        .insert_resource(WinitSettings::game())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "DOOM".into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: false,
                resolution: WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .insert_resource(CommandLineArgs(args))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DoomColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl DoomColor {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(1, 1, 1);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub struct DoomPalette {
    array: [DoomColor; 256],
}

impl DoomPalette {
    pub fn new(array: [DoomColor; 256]) -> Self {
        Self { array }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    args: Res<CommandLineArgs>,
) -> Result {
    if args.0.len() < 2 {
        return Err(anyhow!("No WAD file specified!").into());
    }

    let wad = WadFile::load(&args.0[1])?;

    let playpal = wad.load_lump(wad.find_lump(WadName::from("PLAYPAL")).unwrap());
    let first_palette = &playpal[0..768];
    let mut palette_array = [DoomColor::BLACK; 256];
    let mut palette_formatted = Vec::with_capacity(1024);
    for i in 0..first_palette.len() / 3 {
        let r = first_palette[i * 3];
        let g = first_palette[i * 3 + 1];
        let b = first_palette[i * 3 + 2];
        palette_formatted.push(r);
        palette_formatted.push(g);
        palette_formatted.push(b);
        palette_formatted.push(255);
        palette_array[i] = DoomColor::new(r, g, b);
    }
    let doom_palette = DoomPalette::new(palette_array);
    let mut palette_image = Image::new(
        Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        palette_formatted,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    palette_image.sampler = bevy::image::ImageSampler::nearest();
    let palette_handle = images.add(palette_image);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(64.0, 64.0, 64.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            base_color_texture: Some(palette_handle),
            ..default()
        })),
    ));

    let map = Map::load(&wad, WadName::from("E1M2")).unwrap();

    let sector = map.sectors[0];
    let floor_flat = wad.load_lump(wad.find_lump(sector.floor_texture).unwrap());
    let mut image_data = Vec::with_capacity(64 * 64 * 4);
    for pixel in floor_flat {
        let color = doom_palette.array[*pixel as usize];
        image_data.push(color.r);
        image_data.push(color.g);
        image_data.push(color.b);
        image_data.push(255);
    }
    let mut flat_image = Image::new(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        image_data,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    flat_image.sampler = bevy::image::ImageSampler::nearest();
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(64.0, 64.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(images.add(flat_image)),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 100.0, 0.0),
    ));

    let unlit_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        // base_color_texture: Some(palette_handle),
        // cull_mode: None,
        ..default()
    });

    let map_mesh = map.build_mesh(&wad, &doom_palette);

    commands.spawn((
        Mesh3d(meshes.add(map_mesh.walls)),
        MeshMaterial3d(unlit_material.clone()),
    ));
    for (mesh, image) in map_mesh.floors {
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(image)),
                unlit: true,
                ..default()
            })),
        ));
    }
    for (mesh, image) in map_mesh.ceilings {
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(image)),
                unlit: true,
                ..default()
            })),
        ));
    }

    let player_thing = map.things.iter().find(|t| t.thing_type == 1).unwrap();

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            near: 4.0,
            far: 16000.0,
            ..Default::default()
        }),
        Transform::from_rotation(Quat::from_rotation_y(player_thing.angle)).with_translation(
            Vec3::new(
                player_thing.position.x as f32,
                56.0, // Add sector floor height
                player_thing.position.y as f32,
            ),
        ),
        FreeCamera {
            sensitivity: 0.2,
            friction: 25.0,
            walk_speed: 100.0,
            run_speed: 300.0,
            ..default()
        },
    ));

    commands.insert_resource(wad);

    Ok(())
}

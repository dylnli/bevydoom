mod map;
mod wad;

use crate::{
    map::Map,
    wad::{WadFile, types::WadName},
};

use anyhow::anyhow;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::WindowResolution,
    winit::WinitSettings,
};

#[derive(Resource)]
struct CommandLineArgs(Vec<String>);

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

const GAME_WIDTH: u32 = 800;
const GAME_HEIGHT: u32 = 500;

const PIXEL_PERFECT_LAYERS: RenderLayers = RenderLayers::layer(0);
const HIGH_RES_LAYERS: RenderLayers = RenderLayers::layer(1);

// Low-resolution texture that contains the pixel perfect world that is rendered to high res camera
#[derive(Component)]
struct Canvas;

// Camera that renders the pixel perfect world to canvas
#[derive(Component)]
struct GameCamera;

// Camera that renders the canvas and other HIGH_RES_LAYERS things to screen
#[derive(Component)]
struct OuterCamera;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    App::new()
        .insert_resource(WinitSettings::game())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "DOOM".into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: false,
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .insert_resource(CommandLineArgs(args))
        .add_systems(Startup, (setup_wad, setup_map, setup_camera).chain())
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

pub struct DoomPalette([DoomColor; 256]);

fn setup_wad(mut commands: Commands, args: Res<CommandLineArgs>) -> Result {
    if args.0.len() < 2 {
        return Err(anyhow!("No WAD file specified!").into());
    }

    let wad = WadFile::load(&args.0[1])?;

    commands.insert_resource(wad);

    Ok(())
}

fn setup_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    wad: Res<WadFile>,
) -> Result {
    let playpal = wad.load_lump(wad.find_lump(WadName::from_slice(b"PLAYPAL"))?);
    let first_palette = &playpal[0..768];
    let mut palette_array = [DoomColor::BLACK; 256];
    for i in 0..first_palette.len() / 3 {
        let r = first_palette[i * 3];
        let g = first_palette[i * 3 + 1];
        let b = first_palette[i * 3 + 2];
        palette_array[i] = DoomColor::new(r, g, b);
    }
    let doom_palette = DoomPalette(palette_array);

    let map = Map::load(&wad, WadName::from_slice(b"E1M2"))?;

    let unlit_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        // cull_mode: None,
        ..default()
    });

    let map_mesh = map.build_mesh(&wad, &doom_palette);

    commands.spawn((
        Mesh3d(meshes.add(map_mesh.walls)),
        MeshMaterial3d(unlit_material.clone()),
    ));
    for textured_mesh in map_mesh.floors {
        commands.spawn((
            Mesh3d(meshes.add(textured_mesh.mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(textured_mesh.texture)),
                unlit: true,
                ..default()
            })),
        ));
    }
    for textured_mesh in map_mesh.ceilings {
        commands.spawn((
            Mesh3d(meshes.add(textured_mesh.mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(images.add(textured_mesh.texture)),
                unlit: true,
                ..default()
            })),
        ));
    }

    commands.insert_resource(map);

    Ok(())
}

fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>, map: Res<Map>) {
    let canvas_size = Extent3d {
        width: GAME_WIDTH,
        height: GAME_HEIGHT,
        ..default()
    };

    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // Fill image.data with zeroes
    canvas.resize(canvas_size);

    let image_handle = images.add(canvas);

    let player_thing = map.things.iter().find(|t| t.thing_type == 1).unwrap();

    commands.spawn((
        GameCamera,
        Camera3d::default(),
        Camera {
            // Render before the "main pass" camera
            order: -1,
            ..default()
        },
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
            friction: 10.0,
            walk_speed: 200.0,
            run_speed: 300.0,
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
        Msaa::Off,
        PIXEL_PERFECT_LAYERS,
    ));

    commands.spawn((
        Canvas,
        Sprite {
            image: image_handle,
            custom_size: Some(Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32)),
            image_mode: SpriteImageMode::Auto,
            ..default()
        },
        HIGH_RES_LAYERS,
    ));

    commands.spawn((OuterCamera, Camera2d, Msaa::Off, HIGH_RES_LAYERS));
}

mod wad;

use anyhow::anyhow;
use bevy::{prelude::*, window::WindowResolution, winit::WinitSettings};

use crate::wad::WadFile;

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
        .insert_resource(CommandLineArgs(args))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_vertices)
        .run();
}

#[derive(Resource)]
struct Vertices(Vec<wad::types::Vertex>);

fn setup(mut commands: Commands, args: Res<CommandLineArgs>) -> Result {
    if args.0.len() < 2 {
        return Err(anyhow!("No WAD file specified!").into());
    }

    let wad = WadFile::load(&args.0[1])?;

    let map = wad.load_map("E1M3").unwrap();
    
    commands.insert_resource(wad);

    commands.insert_resource(Vertices(map.vertices));

    commands.spawn(Camera2d);

    Ok(())
}

fn draw_vertices(
    mut gizmos: Gizmos,
    vertices: Res<Vertices>,
) {
    for vertex in &vertices.0 {
        gizmos.circle_2d(vec2(vertex.0.x as f32 / 20.0, vertex.0.y as f32 / 20.0), 1.0, Color::WHITE);
    }
}

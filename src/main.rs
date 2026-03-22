mod map;
mod wad;

use anyhow::anyhow;
use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    args: Res<CommandLineArgs>,
) -> Result {
    if args.0.len() < 2 {
        return Err(anyhow!("No WAD file specified!").into());
    }

    let wad = WadFile::load(&args.0[1])?;

    let map = Map::load(&wad, WadName::from("E1M1")).unwrap();

    commands.insert_resource(wad);

    let unlit_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(map.build_mesh())),
        MeshMaterial3d(unlit_material),
    ));

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
                56.0,
                player_thing.position.y as f32,
            ),
        ),
        FreeCamera {
            sensitivity: 0.2,
            friction: 25.0,
            walk_speed: 50.0,
            run_speed: 150.0,
            ..default()
        },
    ));

    Ok(())
}

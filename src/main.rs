mod wad;

use anyhow::anyhow;
use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
    window::WindowResolution,
    winit::WinitSettings,
};

use crate::wad::{WadFile, types::Map};

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

    let map = wad.load_map("E1M2").unwrap();

    commands.insert_resource(wad);

    let unlit_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(create_map_mesh(&map))),
        MeshMaterial3d(unlit_material),
    ));

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            near: 4.0,
            far: 16000.0,
            ..Default::default()
        }),
        Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
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

fn create_map_mesh(map: &Map) -> Mesh {
    let mut vertices = Vec::with_capacity(map.linedefs.len() * 4);
    let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(map.linedefs.len() * 4);
    let mut indices = Vec::with_capacity(map.linedefs.len() * 6);

    for linedef in &map.linedefs {
        if !linedef.is_two_sided() {
            let start_vertex = map.vertices[linedef.start_vertex as usize];
            let end_vertex = map.vertices[linedef.end_vertex as usize];
            let sidedef = map.sidedefs[linedef.front_sidedef as usize];
            let sector = map.sectors[sidedef.sector as usize];

            let wall_vertices = [
                [
                    start_vertex.0.x as f32,
                    sector.ceiling_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    start_vertex.0.x as f32,
                    sector.floor_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    sector.floor_height as f32,
                    end_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    sector.ceiling_height as f32,
                    end_vertex.0.y as f32,
                ],
            ];
            let wall_vertex_colors = [[1.0, 0.0, 0.0, 1.0]; 4];
            let start_index = vertices.len() as u32;
            let wall_indices = [
                start_index + 0,
                start_index + 1,
                start_index + 3,
                start_index + 1,
                start_index + 2,
                start_index + 3,
            ];

            vertices.extend(wall_vertices);
            vertex_colors.extend(wall_vertex_colors);
            indices.extend(wall_indices);
        } else {
            let start_vertex = map.vertices[linedef.start_vertex as usize];
            let end_vertex = map.vertices[linedef.end_vertex as usize];
            let front_sidedef = map.sidedefs[linedef.front_sidedef as usize];
            let front_sector = map.sectors[front_sidedef.sector as usize];
            let back_sidedef = map.sidedefs[linedef.back_sidedef as usize];
            let back_sector = map.sectors[back_sidedef.sector as usize];

            let top_wall_vertices = [
                [
                    start_vertex.0.x as f32,
                    front_sector.ceiling_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    start_vertex.0.x as f32,
                    back_sector.ceiling_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    back_sector.ceiling_height as f32,
                    end_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    front_sector.ceiling_height as f32,
                    end_vertex.0.y as f32,
                ],
            ];
            let top_wall_vertex_colors = [[0.0, 1.0, 0.0, 1.0]; 4];
            let top_start_index = vertices.len() as u32;
            let top_wall_indices = [
                top_start_index + 0,
                top_start_index + 1,
                top_start_index + 3,
                top_start_index + 1,
                top_start_index + 2,
                top_start_index + 3,
            ];

            vertices.extend(top_wall_vertices);
            vertex_colors.extend(top_wall_vertex_colors);
            indices.extend(top_wall_indices);

            let bottom_wall_vertices = [
                [
                    start_vertex.0.x as f32,
                    back_sector.floor_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    start_vertex.0.x as f32,
                    front_sector.floor_height as f32,
                    start_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    front_sector.floor_height as f32,
                    end_vertex.0.y as f32,
                ],
                [
                    end_vertex.0.x as f32,
                    back_sector.floor_height as f32,
                    end_vertex.0.y as f32,
                ],
            ];
            let bottom_wall_vertex_colors = [[0.0, 0.0, 1.0, 1.0]; 4];
            let bottom_start_index = vertices.len() as u32;
            let bottom_wall_indices = [
                bottom_start_index + 0,
                bottom_start_index + 1,
                bottom_start_index + 3,
                bottom_start_index + 1,
                bottom_start_index + 2,
                bottom_start_index + 3,
            ];

            vertices.extend(bottom_wall_vertices);
            vertex_colors.extend(bottom_wall_vertex_colors);
            indices.extend(bottom_wall_indices);
        }
    }

    use bevy::{
        asset::RenderAssetUsages,
        mesh::{Indices, PrimitiveTopology},
    };

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors)
    .with_inserted_indices(Indices::U32(indices))
}

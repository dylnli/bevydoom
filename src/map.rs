use crate::{
    DoomPalette,
    wad::{WadFile, types::*},
};

use bevy::{
    asset::RenderAssetUsages,
    ecs::resource::Resource,
    image::Image,
    math::IVec2,
    mesh::{Indices, Mesh, PrimitiveTopology},
};

#[derive(Resource, Clone, Debug)]
pub struct Map {
    pub name: WadName,
    pub things: Vec<Thing>,
    pub linedefs: Vec<Linedef>,
    pub sidedefs: Vec<Sidedef>,
    pub vertices: Vec<Vertex>,
    pub sectors: Vec<Sector>,
}

#[derive(Clone)]
pub struct MapMesh {
    pub walls: Vec<TexturedMesh>,
    pub floors: Vec<TexturedMesh>,
    pub ceilings: Vec<TexturedMesh>,
}

#[derive(Clone)]
pub struct TexturedMesh {
    pub mesh: Mesh,
    pub texture: Image,
}

impl TexturedMesh {
    pub fn new(mesh: Mesh, texture: Image) -> Self {
        Self { mesh, texture }
    }
}

impl Map {
    pub fn load(wad: &WadFile, name: WadName) -> anyhow::Result<Self> {
        let map_index = wad.find_lump(name)?;

        let things = bytes_to_vec::<Thing, RawThing>(wad.load_lump(map_index + 1))?;
        let linedefs = bytes_to_vec::<Linedef, RawLinedef>(wad.load_lump(map_index + 2))?;
        let sidedefs = bytes_to_vec::<Sidedef, RawSidedef>(wad.load_lump(map_index + 3))?;
        let vertices = bytes_to_vec::<Vertex, RawVertex>(wad.load_lump(map_index + 4))?;
        let sectors = bytes_to_vec::<Sector, RawSector>(wad.load_lump(map_index + 8))?;

        Ok(Map {
            name,
            things,
            linedefs,
            sidedefs,
            vertices,
            sectors,
        })
    }

    pub fn build_mesh(
        &self,
        wad: &WadFile,
        textures: &[Texture],
        palette: &DoomPalette,
    ) -> anyhow::Result<MapMesh> {
        let walls = self.build_walls_mesh(textures, palette);
        let (floors, ceilings) = self.build_floors_ceilings_meshes(wad, palette)?;
        Ok(MapMesh {
            walls,
            floors,
            ceilings,
        })
    }

    fn build_walls_mesh(&self, textures: &[Texture], palette: &DoomPalette) -> Vec<TexturedMesh> {
        // let mut vertices = Vec::with_capacity(self.linedefs.len() * 4);
        let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(self.linedefs.len() * 4);
        // let mut indices = Vec::with_capacity(self.linedefs.len() * 6);

        let mut add_quad = |quad_vertices: [[f32; 3]; 4], color: [f32; 4]| {
            let start_index = 0;
            // let start_index = vertices.len() as u32;
            (
                quad_vertices,
                [color; 4],
                [
                    start_index + 0,
                    start_index + 1,
                    start_index + 3,
                    start_index + 1,
                    start_index + 2,
                    start_index + 3,
                ],
            )
        };

        let mut add_wall = |right_vertex: IVec2,
                            left_vertex: IVec2,
                            top_height: i32,
                            bottom_height: i32,
                            color: [f32; 4]| {
            let new_vertices = [
                [
                    right_vertex.x as f32,
                    top_height as f32,
                    right_vertex.y as f32,
                ],
                [
                    right_vertex.x as f32,
                    bottom_height as f32,
                    right_vertex.y as f32,
                ],
                [
                    left_vertex.x as f32,
                    bottom_height as f32,
                    left_vertex.y as f32,
                ],
                [
                    left_vertex.x as f32,
                    top_height as f32,
                    left_vertex.y as f32,
                ],
            ];
            add_quad(new_vertices, color)
        };

        let mut meshes = Vec::with_capacity(self.linedefs.len());

        for linedef in &self.linedefs {
            if let Some(back_sidedef_i) = linedef.back_sidedef_i {
                // Two sided
                let start_vertex = self.vertices[linedef.start_vertex_i as usize];
                let end_vertex = self.vertices[linedef.end_vertex_i as usize];
                let front_sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
                let front_sector = self.sectors[front_sidedef.sector_i as usize];
                let back_sidedef = self.sidedefs[back_sidedef_i as usize];
                let back_sector = self.sectors[back_sidedef.sector_i as usize];

                // TODO: Handle dynamic geometry
                // Top wall
                if front_sector.ceiling_height != back_sector.ceiling_height {
                    if front_sector.ceiling_height > back_sector.ceiling_height {
                        add_wall(
                            start_vertex.0,
                            end_vertex.0,
                            front_sector.ceiling_height,
                            back_sector.ceiling_height,
                            [0.0, 1.0, 0.0, 1.0],
                        );
                    } else {
                        add_wall(
                            end_vertex.0,
                            start_vertex.0,
                            back_sector.ceiling_height,
                            front_sector.ceiling_height,
                            [0.0, 1.0, 0.0, 1.0],
                        );
                    }
                }

                // Bottom wall
                if front_sector.floor_height != back_sector.floor_height {
                    if front_sector.floor_height < back_sector.floor_height {
                        add_wall(
                            start_vertex.0,
                            end_vertex.0,
                            back_sector.floor_height,
                            front_sector.floor_height,
                            [0.0, 0.0, 1.0, 1.0],
                        );
                    } else {
                        add_wall(
                            end_vertex.0,
                            start_vertex.0,
                            front_sector.floor_height,
                            back_sector.floor_height,
                            [0.0, 0.0, 1.0, 1.0],
                        );
                    }
                }
            } else {
                // One sided
                let start_vertex = self.vertices[linedef.start_vertex_i as usize];
                let end_vertex = self.vertices[linedef.end_vertex_i as usize];
                let sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
                let sector = self.sectors[sidedef.sector_i as usize];

                let (vertices, vertex_colors, indices) = add_wall(
                    start_vertex.0,
                    end_vertex.0,
                    sector.ceiling_height,
                    sector.floor_height,
                    [1.0, 1.0, 1.0, 1.0],
                );
                let image = if let Some(texture) =
                    textures.iter().find(|t| t.name == sidedef.middle_texture)
                {
                    texture.to_image(palette)
                } else {
                    Image::new(
                        bevy::render::render_resource::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        vec![255, 0, 255, 0],
                        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                        bevy::asset::RenderAssetUsages::RENDER_WORLD,
                    )
                };
                meshes.push(TexturedMesh::new(
                    Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::from(vertices))
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
                    )
                    .with_inserted_indices(Indices::U32(Vec::from(indices))),
                    image,
                ))
            }
        }

        meshes

        // Mesh::new(
        //     PrimitiveTopology::TriangleList,
        //     RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        // )
        // .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        // .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors)
        // .with_inserted_indices(Indices::U32(indices))
    }

    fn build_floors_ceilings_meshes(
        &self,
        wad: &WadFile,
        palette: &DoomPalette,
    ) -> anyhow::Result<(Vec<TexturedMesh>, Vec<TexturedMesh>)> {
        let num_sectors = self.sectors.len();

        #[derive(Clone, Copy)]
        struct CollectedEdge {
            start_vertex_i: u32,
            end_vertex_i: u32,
            visited: bool,
        }
        // Array of array for each sector, each sector array is array of edges + if visited
        let mut collected_sector_edges: Vec<Vec<CollectedEdge>> = vec![Vec::new(); num_sectors];
        for linedef in &self.linedefs {
            let front_sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
            let front_sector_i = front_sidedef.sector_i as usize;
            collected_sector_edges[front_sector_i].push(CollectedEdge {
                start_vertex_i: linedef.start_vertex_i,
                end_vertex_i: linedef.end_vertex_i,
                visited: false,
            });

            if let Some(back_sidedef_i) = linedef.back_sidedef_i {
                let back_sidedef = self.sidedefs[back_sidedef_i as usize];
                let back_sector_i = back_sidedef.sector_i as usize;
                collected_sector_edges[back_sector_i].push(CollectedEdge {
                    start_vertex_i: linedef.end_vertex_i,
                    end_vertex_i: linedef.start_vertex_i,
                    visited: false,
                });
            }
        }

        // Array of array for each sector, each sector array is array of loops (array of vertex indices);
        let mut sector_loops_vec: Vec<Vec<Vec<u32>>> = vec![Vec::new(); num_sectors];
        for sector_i in 0..num_sectors {
            let sector_edges = &mut collected_sector_edges[sector_i];
            let num_edges = sector_edges.len();
            let sector_loops = &mut sector_loops_vec[sector_i];

            for edge_i in 0..num_edges {
                let start_edge = sector_edges[edge_i];
                if start_edge.visited {
                    continue;
                }
                sector_edges[edge_i].visited = true;

                // Encountering new loop
                let mut new_loop = vec![start_edge.start_vertex_i, start_edge.end_vertex_i];

                // Find all connected edges
                let search_edges = &mut sector_edges[edge_i + 1..];
                let search_edges_len = search_edges.len();

                // search_edges_len ^ 2 is max number of iterations needed
                for _ in 0..search_edges_len {
                    for rest_i in 0..search_edges_len {
                        let new_edge = &mut search_edges[rest_i];
                        if new_edge.visited {
                            continue;
                        }
                        if new_edge.start_vertex_i != *new_loop.last().unwrap() {
                            // Not connected
                            continue;
                        }

                        // New connected edge
                        new_edge.visited = true;

                        if new_edge.end_vertex_i == start_edge.start_vertex_i {
                            // Closed loop
                            break;
                        }

                        new_loop.push(new_edge.end_vertex_i);
                    }
                }

                sector_loops.push(new_loop);
            }
        }

        // Find outer loops, move to first loop
        for sector_loops in &mut sector_loops_vec {
            fn cmp_more_extreme(a: IVec2, b: IVec2) -> std::cmp::Ordering {
                let cmpge = a.cmpge(b);
                if cmpge.x && cmpge.y {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            }

            let mut most_extreme = IVec2::MIN;
            let mut outer_loop_i = 0;
            for (i, l) in sector_loops.iter().enumerate() {
                let extreme_i = l
                    .iter()
                    .max_by(|&&a, &&b| {
                        let vertex_a = self.vertices[a as usize];
                        let vertex_b = self.vertices[b as usize];
                        cmp_more_extreme(vertex_a.0, vertex_b.0)
                    })
                    .unwrap();
                let extreme = self.vertices[*extreme_i as usize].0;
                if cmp_more_extreme(extreme, most_extreme) == std::cmp::Ordering::Greater {
                    most_extreme = extreme;
                    outer_loop_i = i;
                }
            }

            // Swap loops
            sector_loops.swap(0, outer_loop_i);
        }

        let sector_loops_vertices: Vec<Vec<Vec<[f32; 2]>>> = sector_loops_vec
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|vv| {
                        vv.into_iter()
                            .map(|i| {
                                [
                                    self.vertices[i as usize].0.x as f32,
                                    self.vertices[i as usize].0.y as f32,
                                ]
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();

        // Triangulate sector meshes
        use i_triangle::float::triangulation::Triangulation;
        use i_triangle::float::triangulator::Triangulator;
        let mut triangulator = Triangulator::<u32>::default();
        let mut triangulation = Triangulation::with_capacity(10);

        let mut triangulated_sectors = Vec::with_capacity(num_sectors);
        for sector_i in 0..num_sectors {
            triangulator.triangulate_into(&sector_loops_vertices[sector_i], &mut triangulation);

            let points = triangulation.points.clone();
            let uvs: Vec<[f32; 2]> = triangulation
                .points
                .iter()
                .map(|p| [p[0] / 64.0, p[1] / 64.0])
                .collect();
            let indices = triangulation.indices.clone();

            triangulated_sectors.push((points, uvs, indices));
        }

        // Batch floor and ceiling meshes by texture
        let mut floors_out = Vec::with_capacity(num_sectors);
        let mut ceilings_out = Vec::with_capacity(num_sectors);

        let mut image_sampler_descriptor = bevy::image::ImageSamplerDescriptor::nearest();
        image_sampler_descriptor.set_address_mode(bevy::image::ImageAddressMode::Repeat);
        let image_sampler = bevy::image::ImageSampler::Descriptor(image_sampler_descriptor);

        let mut floors_visited = vec![false; num_sectors];
        let mut ceilings_visited = vec![false; num_sectors];

        for sector_i in 0..num_sectors {
            let sector = self.sectors[sector_i];

            // Floor mesh
            if !floors_visited[sector_i] {
                floors_visited[sector_i] = true;

                let mut vertices: Vec<[f32; 3]> = Vec::new();
                let mut uvs: Vec<[f32; 2]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                let floor_texture_name = sector.floor_texture;

                // Visit all sectors with the same floor texture
                for new_i in sector_i..num_sectors {
                    let new_sector = self.sectors[new_i];
                    if new_sector.floor_texture != floor_texture_name {
                        continue;
                    }
                    floors_visited[new_i] = true;

                    let floor_height = new_sector.floor_height as f32;
                    let new_vertices: Vec<[f32; 3]> = triangulated_sectors[new_i]
                        .0
                        .iter()
                        .map(|v| [v[0], floor_height, v[1]])
                        .collect();
                    let new_uvs: Vec<[f32; 2]> = triangulated_sectors[new_i].1.clone();
                    let index_offset = vertices.len() as u32;
                    let new_indices: Vec<u32> = triangulated_sectors[new_i]
                        .2
                        .iter()
                        .map(|i| i + index_offset)
                        .rev()
                        .collect();

                    vertices.extend(new_vertices);
                    uvs.extend(new_uvs);
                    indices.extend(new_indices);
                }

                let floor_mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_indices(Indices::U32(indices));

                // Floor texture
                let floor_flat = wad.load_flat(wad.find_flat(floor_texture_name)?)?;
                let mut floor_image = floor_flat.to_image(palette);
                floor_image.sampler = image_sampler.clone();

                // Add floor
                floors_out.push(TexturedMesh::new(floor_mesh, floor_image));
            }

            // Ceiling mesh
            if !ceilings_visited[sector_i] {
                ceilings_visited[sector_i] = true;

                let mut vertices: Vec<[f32; 3]> = Vec::new();
                let mut uvs: Vec<[f32; 2]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                let ceiling_texture_name = sector.ceiling_texture;

                // Visit all sectors with the same ceiling texture
                for new_i in sector_i..num_sectors {
                    let new_sector = self.sectors[new_i];
                    if new_sector.ceiling_texture != ceiling_texture_name {
                        continue;
                    }
                    ceilings_visited[new_i] = true;

                    let ceiling_height = new_sector.ceiling_height as f32;
                    let new_vertices: Vec<[f32; 3]> = triangulated_sectors[new_i]
                        .0
                        .iter()
                        .map(|v| [v[0], ceiling_height, v[1]])
                        .collect();
                    let new_uvs: Vec<[f32; 2]> = triangulated_sectors[new_i].1.clone();
                    let index_offset = vertices.len() as u32;
                    let new_indices: Vec<u32> = triangulated_sectors[new_i]
                        .2
                        .iter()
                        .map(|i| i + index_offset)
                        .collect();

                    vertices.extend(new_vertices);
                    uvs.extend(new_uvs);
                    indices.extend(new_indices);
                }

                let ceiling_mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
                .with_inserted_indices(Indices::U32(indices));

                // Ceiling texture
                let ceiling_flat = wad.load_flat(wad.find_flat(ceiling_texture_name)?)?;
                let mut ceiling_image = ceiling_flat.to_image(palette);
                ceiling_image.sampler = image_sampler.clone();

                // Add ceiling
                ceilings_out.push(TexturedMesh::new(ceiling_mesh, ceiling_image));
            }
        }

        Ok((floors_out, ceilings_out))
    }
}

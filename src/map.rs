use bevy::{math::IVec2, mesh::Mesh};

use crate::wad::{WadFile, types::*};

#[derive(Debug, Clone)]
pub struct Map {
    pub name: WadName,
    pub things: Vec<Thing>,
    pub linedefs: Vec<Linedef>,
    pub sidedefs: Vec<Sidedef>,
    pub vertices: Vec<Vertex>,
    pub segs: Vec<Seg>,
    pub subsectors: Vec<Subsector>,
    pub sectors: Vec<Sector>,
}

impl Map {
    pub fn load(wad: &WadFile, name: WadName) -> Option<Self> {
        let map_index = wad.find_lump(name)?;

        let things = WadFile::bytes_to_vec::<Thing, RawThing>(wad.load_lump(map_index + 1));
        let linedefs = WadFile::bytes_to_vec::<Linedef, RawLinedef>(wad.load_lump(map_index + 2));
        let sidedefs = WadFile::bytes_to_vec::<Sidedef, RawSidedef>(wad.load_lump(map_index + 3));
        let vertices = WadFile::bytes_to_vec::<Vertex, RawVertex>(wad.load_lump(map_index + 4));
        let segs = WadFile::bytes_to_vec::<Seg, RawSeg>(wad.load_lump(map_index + 5));
        let subsectors =
            WadFile::bytes_to_vec::<Subsector, RawSubsector>(wad.load_lump(map_index + 6));
        let sectors = WadFile::bytes_to_vec::<Sector, RawSector>(wad.load_lump(map_index + 8));

        Some(Map {
            name,
            things,
            linedefs,
            sidedefs,
            vertices,
            segs,
            subsectors,
            sectors,
        })
    }

    pub fn build_mesh(&self) -> Mesh {
        let mut vertices = Vec::with_capacity(self.linedefs.len() * 4);
        let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(self.linedefs.len() * 4);
        let mut indices = Vec::with_capacity(self.linedefs.len() * 6);

        // Walls

        let mut add_quad = |quad_vertices: [[f32; 3]; 4], color: [f32; 4]| {
            let start_index = vertices.len() as u32;
            vertices.extend(quad_vertices);
            vertex_colors.extend([color; 4]);
            indices.extend([
                start_index + 0,
                start_index + 1,
                start_index + 3,
                start_index + 1,
                start_index + 2,
                start_index + 3,
            ]);
        };

        let mut add_wall = |right_vertex: IVec2,
                            left_vertex: IVec2,
                            top_height: i32,
                            bottom_height: i32,
                            color: [f32; 4]| {
            let vertices = [
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
            add_quad(vertices, color);
        };

        for linedef in &self.linedefs {
            if let Some(back_sidedef_i) = linedef.back_sidedef_i {
                // Two sided
                let start_vertex = self.vertices[linedef.start_vertex_i as usize];
                let end_vertex = self.vertices[linedef.end_vertex_i as usize];
                let front_sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
                let front_sector = self.sectors[front_sidedef.sector_i as usize];
                let back_sidedef = self.sidedefs[back_sidedef_i as usize];
                let back_sector = self.sectors[back_sidedef.sector_i as usize];

                // TODO: Generate correct winding based on height and two quads per wall for dynamic geometry
                add_wall(
                    start_vertex.0,
                    end_vertex.0,
                    front_sector.ceiling_height,
                    back_sector.ceiling_height,
                    [0.0, 1.0, 0.0, 1.0],
                );
                add_wall(
                    start_vertex.0,
                    end_vertex.0,
                    back_sector.floor_height,
                    front_sector.floor_height,
                    [0.0, 0.0, 1.0, 1.0],
                );
            } else {
                // One sided
                let start_vertex = self.vertices[linedef.start_vertex_i as usize];
                let end_vertex = self.vertices[linedef.end_vertex_i as usize];
                let sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
                let sector = self.sectors[sidedef.sector_i as usize];

                add_wall(
                    start_vertex.0,
                    end_vertex.0,
                    sector.ceiling_height,
                    sector.floor_height,
                    [1.0, 0.0, 0.0, 1.0],
                );
            }
        }

        // Floors and ceilings

        let num_sectors = self.sectors.len();

        // Array of array for each sector, each sector array is array of edges + if visited
        let mut collected_sector_edges: Vec<Vec<(u32, u32, bool)>> = vec![Vec::new(); num_sectors];
        for linedef in &self.linedefs {
            let front_sidedef = self.sidedefs[linedef.front_sidedef_i as usize];
            let front_sector_i = front_sidedef.sector_i as usize;
            collected_sector_edges[front_sector_i].push((
                linedef.start_vertex_i,
                linedef.end_vertex_i,
                false,
            ));

            if let Some(back_sidedef_i) = linedef.back_sidedef_i {
                let back_sidedef = self.sidedefs[back_sidedef_i as usize];
                let back_sector_i = back_sidedef.sector_i as usize;
                collected_sector_edges[back_sector_i].push((
                    linedef.end_vertex_i,
                    linedef.start_vertex_i,
                    false,
                ));
            }
        }

        println!("{:?}", collected_sector_edges);

        // Array of array for each sector, each sector array is array of loops (array of vertex indices);
        let mut sector_loops_vec: Vec<Vec<Vec<u32>>> = vec![Vec::new(); num_sectors];
        for sector_i in 0..num_sectors {
            let sector_edges = &mut collected_sector_edges[sector_i];
            println!("NEW SECTOR {:?}", sector_edges);
            let sector_loops = &mut sector_loops_vec[sector_i];
            let num_edges = sector_edges.len();
            bevy::log::info!("NUM EDGES {}", num_edges);
            for i in 0..num_edges {
                let start_edge = sector_edges[i];
                if start_edge.2 {
                    // Already visited
                    continue;
                }
                sector_edges[i].2 = true;

                println!("START EDGE {} {}", start_edge.0, start_edge.1);

                // Encountering new loop
                let mut new_loop = Vec::new();
                new_loop.extend([start_edge.0, start_edge.1]);

                // Find all connected edges
                let rest_edges = &mut sector_edges[i + 1..];
                let rest_edges_len = rest_edges.len();
                println!("SEARCH START FROM {}", i + 1);
                for _ in 0..rest_edges_len {
                    for rest_i in 0..rest_edges_len {
                        let new_edge = &mut rest_edges[rest_i];
                        if new_edge.2 {
                            continue;
                        }

                        if new_edge.0 == *new_loop.last().unwrap() {
                            println!("NEW EDGE {} {}", new_edge.0, new_edge.1);
                            bevy::log::info!("FOUND NEW EDGE");
                            new_edge.2 = true;
                            // New edge
                            if new_edge.1 == start_edge.0 {
                                // Closed loop
                                break;
                            }

                            new_loop.push(new_edge.1);
                        }
                    }
                }

                sector_loops.push(new_loop);
                println!("ONE LOOP");
                println!("{:?}", sector_edges);
            }
            println!("ONE SECTOR");
            println!("{:?}", sector_edges);
        }
        println!("DONE!");
        println!("{:?}", sector_loops_vec);

        // Find outer loops, move to first loop
        for sector_loops in &mut sector_loops_vec {
            fn cmp_more_bottom_right(a: IVec2, b: IVec2) -> std::cmp::Ordering {
                if a.y != b.y {
                    a.y.cmp(&b.y)
                } else {
                    a.x.cmp(&b.x)
                }
            }

            let bottom_right_most = IVec2::MIN;
            let mut outer_loop_i = 0;
            for (i, l) in sector_loops.iter().enumerate() {
                let extreme_i = l
                    .iter()
                    .max_by(|&&a, &&b| {
                        let vertex_a = self.vertices[a as usize];
                        let vertex_b = self.vertices[b as usize];
                        cmp_more_bottom_right(vertex_a.0, vertex_b.0)
                    })
                    .unwrap();
                let extreme = self.vertices[*extreme_i as usize].0;
                if cmp_more_bottom_right(extreme, bottom_right_most) == std::cmp::Ordering::Greater
                {
                    outer_loop_i = i;
                }
            }

            // Swap loops
            sector_loops.swap(0, outer_loop_i);
        }

        let sector_loops_vertices_vec: Vec<Vec<Vec<[f32; 2]>>> = sector_loops_vec
            .iter()
            .map(|v| {
                v.iter()
                    .map(|vv| {
                        vv.iter()
                            .map(|&i| {
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

        use i_triangle::float::triangulatable::Triangulatable;
        for sector_i in 0..num_sectors {
            let triangulation = sector_loops_vertices_vec[sector_i]
                .triangulate()
                .to_triangulation::<u32>();

            let floor_height = self.sectors[sector_i].floor_height as f32;
            let floor_vertices: Vec<[f32; 3]> = triangulation
                .points
                .iter()
                .map(|v| [v[0], floor_height, v[1]])
                .collect();
            let floor_start_index = vertices.len();
            let floor_indices: Vec<u32> = triangulation
                .indices
                .iter()
                .map(|i| i + floor_start_index as u32)
                .rev()
                .collect();

            vertex_colors.extend(vec![[1.0, 1.0, 0.0, 1.0]; floor_vertices.len()]);
            vertices.extend(floor_vertices);
            indices.extend(floor_indices);

            let ceiling_height = self.sectors[sector_i].ceiling_height as f32;
            let ceiling_vertices: Vec<[f32; 3]> = triangulation
                .points
                .into_iter()
                .map(|v| [v[0], ceiling_height, v[1]])
                .collect();
            let ceiling_start_index = vertices.len();
            let ceiling_indices: Vec<u32> = triangulation
                .indices
                .iter()
                .map(|i| i + ceiling_start_index as u32)
                .collect();

            vertex_colors.extend(vec![[1.0, 0.0, 1.0, 1.0]; ceiling_vertices.len()]);
            vertices.extend(ceiling_vertices);
            indices.extend(ceiling_indices);
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
}

use bevy::{
    asset::RenderAssetUsages,
    math::IVec2,
    mesh::{Indices, Mesh, PrimitiveTopology},
};

use crate::wad::{WadFile, types::*};

#[derive(Debug, Clone)]
pub struct Map {
    pub name: WadName,
    pub things: Vec<Thing>,
    pub linedefs: Vec<Linedef>,
    pub sidedefs: Vec<Sidedef>,
    pub vertices: Vec<Vertex>,
    pub sectors: Vec<Sector>,
}

impl Map {
    pub fn load(wad: &WadFile, name: WadName) -> Option<Self> {
        let map_index = wad.find_lump(name)?;

        let things = WadFile::bytes_to_vec::<Thing, RawThing>(wad.load_lump(map_index + 1));
        let linedefs = WadFile::bytes_to_vec::<Linedef, RawLinedef>(wad.load_lump(map_index + 2));
        let sidedefs = WadFile::bytes_to_vec::<Sidedef, RawSidedef>(wad.load_lump(map_index + 3));
        let vertices = WadFile::bytes_to_vec::<Vertex, RawVertex>(wad.load_lump(map_index + 4));
        let sectors = WadFile::bytes_to_vec::<Sector, RawSector>(wad.load_lump(map_index + 8));

        Some(Map {
            name,
            things,
            linedefs,
            sidedefs,
            vertices,
            sectors,
        })
    }

    pub fn build_mesh(&self) -> MapMesh {
        let walls = self.build_walls_mesh();
        let (floors, ceilings) = self.build_floors_ceilings_mesh();
        MapMesh {
            walls,
            floors,
            ceilings,
        }
    }

    fn build_walls_mesh(&self) -> Mesh {
        let mut vertices = Vec::with_capacity(self.linedefs.len() * 4);
        let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(self.linedefs.len() * 4);
        let mut indices = Vec::with_capacity(self.linedefs.len() * 6);

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
            add_quad(new_vertices, color);
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

                // TODO: Handle dynamic geometry
                // Top wall
                if front_sector.ceiling_height != back_sector.ceiling_height {
                    let higher_ceiling_height =
                        front_sector.ceiling_height.max(back_sector.ceiling_height);
                    let lower_ceiling_height =
                        front_sector.ceiling_height.min(back_sector.ceiling_height);
                    add_wall(
                        start_vertex.0,
                        end_vertex.0,
                        higher_ceiling_height,
                        lower_ceiling_height,
                        [0.0, 1.0, 0.0, 1.0],
                    );
                }

                // Bottom wall
                if front_sector.floor_height != back_sector.floor_height {
                    let higher_floor_height =
                        front_sector.floor_height.max(back_sector.floor_height);
                    let lower_floor_height =
                        front_sector.floor_height.min(back_sector.floor_height);
                    add_wall(
                        start_vertex.0,
                        end_vertex.0,
                        higher_floor_height,
                        lower_floor_height,
                        [0.0, 0.0, 1.0, 1.0],
                    );
                }
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

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors)
        .with_inserted_indices(Indices::U32(indices))
    }

    fn build_floors_ceilings_mesh(&self) -> (Mesh, Mesh) {
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

        let mut floors_vertices = Vec::with_capacity(num_sectors * 3);
        let mut floors_vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(num_sectors * 3);
        let mut floors_indices = Vec::with_capacity(num_sectors * 3);
        let mut ceilings_vertices = Vec::with_capacity(num_sectors * 3);
        let mut ceilings_vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(num_sectors * 3);
        let mut ceilings_indices = Vec::with_capacity(num_sectors * 3);

        use i_triangle::float::triangulation::Triangulation;
        use i_triangle::float::triangulator::Triangulator;
        let mut triangulator = Triangulator::<u32>::default();
        let mut triangulation = Triangulation::with_capacity(10);

        for sector_i in 0..num_sectors {
            triangulator.triangulate_into(&sector_loops_vertices[sector_i], &mut triangulation);

            let new_floor_height = self.sectors[sector_i].floor_height as f32;
            let new_floor_vertices = triangulation
                .points
                .iter()
                .map(|v| [v[0], new_floor_height, v[1]]);
            let new_floor_start_index = floors_vertices.len();
            let new_floor_indices = triangulation
                .indices
                .iter()
                .map(|i| i + new_floor_start_index as u32)
                .rev();

            floors_vertex_colors.extend(vec![[1.0, 1.0, 0.0, 1.0]; new_floor_vertices.len()]);
            floors_vertices.extend(new_floor_vertices);
            floors_indices.extend(new_floor_indices);

            let new_ceiling_height = self.sectors[sector_i].ceiling_height as f32;
            let new_ceiling_vertices = triangulation
                .points
                .iter()
                .map(|v| [v[0], new_ceiling_height, v[1]]);
            let new_ceiling_start_index = ceilings_vertices.len();
            let new_ceiling_indices = triangulation
                .indices
                .iter()
                .map(|i| i + new_ceiling_start_index as u32);

            ceilings_vertex_colors.extend(vec![[1.0, 0.0, 1.0, 1.0]; new_ceiling_vertices.len()]);
            ceilings_vertices.extend(new_ceiling_vertices);
            ceilings_indices.extend(new_ceiling_indices);
        }

        let floors_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, floors_vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, floors_vertex_colors)
        .with_inserted_indices(Indices::U32(floors_indices));
        let ceilings_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, ceilings_vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, ceilings_vertex_colors)
        .with_inserted_indices(Indices::U32(ceilings_indices));

        (floors_mesh, ceilings_mesh)
    }
}

pub struct MapMesh {
    pub walls: Mesh,
    pub floors: Mesh,
    pub ceilings: Mesh,
}

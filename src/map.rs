use bevy::{
    math::{IVec2, Vec2},
    mesh::Mesh,
};

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
            if let Some(back_sidedef) = linedef.back_sidedef {
                // Two sided
                let start_vertex = self.vertices[linedef.start_vertex as usize];
                let end_vertex = self.vertices[linedef.end_vertex as usize];
                let front_sidedef = self.sidedefs[linedef.front_sidedef as usize];
                let front_sector = self.sectors[front_sidedef.sector as usize];
                let back_sidedef = self.sidedefs[back_sidedef as usize];
                let back_sector = self.sectors[back_sidedef.sector as usize];

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
                let start_vertex = self.vertices[linedef.start_vertex as usize];
                let end_vertex = self.vertices[linedef.end_vertex as usize];
                let sidedef = self.sidedefs[linedef.front_sidedef as usize];
                let sector = self.sectors[sidedef.sector as usize];

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

        for subsector in &self.subsectors {
            // Get subsector sector
            let seg = self.segs[subsector.first_seg as usize];
            let linedef = self.linedefs[seg.linedef as usize];
            let sidedef_index = if seg.direction == 0 {
                linedef.front_sidedef
            } else {
                linedef.back_sidedef.unwrap()
            };
            let sidedef = self.sidedefs[sidedef_index as usize];
            let sector = self.sectors[sidedef.sector as usize];

            // Get convex hull points
            let mut hull_indices = Vec::with_capacity(subsector.num_segs as usize);
            for i in 0..subsector.num_segs {
                let seg = self.segs[(subsector.first_seg + i) as usize];
                if i == 0 || hull_indices[i as usize - 1] != seg.start_vertex {
                    hull_indices.push(seg.start_vertex);
                }
                if i == 0 || hull_indices[0] != seg.end_vertex {
                    hull_indices.push(seg.end_vertex);
                }
            }

            if hull_indices.len() < 3 {
                continue;
            }

            let mut hull_vertices: Vec<Vec2> = hull_indices
                .iter()
                .map(|&i| {
                    Vec2::new(
                        self.vertices[i as usize].0.x as f32,
                        self.vertices[i as usize].0.y as f32,
                    )
                })
                .collect();

            // Sort hull points CCW
            let (pivot_index, &pivot_point) = hull_vertices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x)))
                .unwrap();
            hull_vertices.swap(0, pivot_index);
            hull_vertices[1..].sort_by(|a, b| {
                let cross = (a - pivot_point).perp_dot(b - pivot_point);
                if cross > 0.0 {
                    std::cmp::Ordering::Less
                } else if cross < 0.0 {
                    std::cmp::Ordering::Greater
                } else {
                    // Collinear, sort by distance from pivot
                    let da = a.distance_squared(pivot_point);
                    let db = b.distance_squared(pivot_point);
                    da.total_cmp(&db)
                }
            });

            // Floor
            let floor_start_index = vertices.len();
            let floor_vertices: Vec<[f32; 3]> = hull_vertices
                .iter()
                .map(|v| [v.x, sector.floor_height as f32, v.y])
                .collect();
            let floor_vertex_colors = vec![[1.0, 1.0, 0.0, 1.0]; floor_vertices.len()];
            let mut floor_indices: Vec<u32> = Vec::with_capacity((floor_vertices.len() - 2) * 3);
            for i in 1..(floor_vertices.len() - 1) {
                floor_indices.push(floor_start_index as u32);
                floor_indices.push((floor_start_index + i) as u32 + 1);
                floor_indices.push((floor_start_index + i) as u32);
            }

            vertices.extend(floor_vertices);
            vertex_colors.extend(floor_vertex_colors);
            indices.extend(floor_indices);

            // Ceiling
            let ceiling_start_index = vertices.len();
            let ceiling_vertices: Vec<[f32; 3]> = hull_vertices
                .iter()
                .map(|v| [v.x, sector.ceiling_height as f32, v.y])
                .collect();
            let ceiling_vertex_colors = vec![[1.0, 0.0, 1.0, 1.0]; ceiling_vertices.len()];
            let mut ceiling_indices: Vec<u32> =
                Vec::with_capacity((ceiling_vertices.len() - 2) * 3);
            for i in 1..(ceiling_vertices.len() - 1) {
                ceiling_indices.push(ceiling_start_index as u32);
                ceiling_indices.push((ceiling_start_index + i) as u32);
                ceiling_indices.push((ceiling_start_index + i) as u32 + 1);
            }

            vertices.extend(ceiling_vertices);
            vertex_colors.extend(ceiling_vertex_colors);
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

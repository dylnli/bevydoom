use bevy::{math::IVec2, mesh::Mesh};

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

                // TODO: Check if sectors are static geometry before culling
                let add_top_wall = front_sector.ceiling_height > back_sector.ceiling_height;
                let add_bottom_wall = front_sector.floor_height < back_sector.floor_height;

                if add_top_wall {
                    add_wall(
                        start_vertex.0,
                        end_vertex.0,
                        front_sector.ceiling_height,
                        back_sector.ceiling_height,
                        [0.0, 1.0, 0.0, 1.0],
                    );
                }
                if add_bottom_wall {
                    add_wall(
                        start_vertex.0,
                        end_vertex.0,
                        back_sector.floor_height,
                        front_sector.floor_height,
                        [0.0, 0.0, 1.0, 1.0],
                    );
                }
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

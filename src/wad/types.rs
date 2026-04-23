use crate::DoomPalette;

use std::fmt;

use anyhow::Context;
use bevy::{image::Image, math::IVec2};
use bytemuck::{Pod, Zeroable};

// WAD TYPE

#[derive(Clone, Copy, Debug)]
pub enum WadType {
    IWad,
    PWad,
}

// WAD NAME

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Zeroable, Pod)]
pub struct WadName([u8; 8]);

impl WadName {
    pub fn from_slice(slice: &[u8]) -> Self {
        let mut buffer = [0u8; 8];
        for (i, &b) in slice.iter().enumerate() {
            if b == 0 || i >= 8 {
                break;
            }
            buffer[i] = b.to_ascii_uppercase();
        }
        Self(buffer)
    }

    pub fn as_slice(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(8);
        &self.0[..end]
    }

    pub fn from_str(s: &str) -> Self {
        Self::from_slice(s.as_bytes())
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_slice()).unwrap_or("badname")
    }
}

impl From<[u8; 8]> for WadName {
    fn from(r: [u8; 8]) -> Self {
        Self::from_slice(&r)
    }
}

impl fmt::Display for WadName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for WadName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name({:?})", self.as_str())
    }
}

// HEADER

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawWadHeader {
    pub magic: [u8; 4],
    pub num_lumps: i32,
    pub directory_offset: i32,
}

// LUMP ENTRY

#[derive(Clone, Copy, Debug)]
pub struct LumpEntry {
    pub name: WadName,
    pub offset: usize,
    pub size: usize,
}

impl LumpEntry {
    pub fn from_raw(r: RawLumpEntry) -> Self {
        Self {
            offset: r.offset as usize,
            size: r.size as usize,
            name: WadName::from_slice(&r.name),
        }
    }
}

impl From<RawLumpEntry> for LumpEntry {
    fn from(r: RawLumpEntry) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawLumpEntry {
    pub offset: i32,
    pub size: i32,
    pub name: [u8; 8],
}

// THING

#[derive(Clone, Copy, Debug)]
pub struct Thing {
    pub position: IVec2,
    pub angle: f32,
    pub thing_type: u32,
    pub flags: u32,
}

impl Thing {
    pub fn from_raw(r: RawThing) -> Self {
        Self {
            position: from_doom_coords(r.x, r.y),
            angle: from_doom_angle(r.angle),
            thing_type: r.thing_type as u32,
            flags: r.flags as u32,
        }
    }
}

impl From<RawThing> for Thing {
    fn from(r: RawThing) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawThing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub thing_type: u16,
    pub flags: u16,
}

// LINEDEF

#[derive(Clone, Copy, Debug)]
pub struct Linedef {
    pub start_vertex_i: u32,
    pub end_vertex_i: u32,
    pub flags: u32,
    pub special: u32,
    pub tag: u32,
    pub front_sidedef_i: u32,
    pub back_sidedef_i: Option<u32>,
}

impl Linedef {
    pub fn from_raw(r: RawLinedef) -> Self {
        Self {
            start_vertex_i: r.start_vertex_i as u32,
            end_vertex_i: r.end_vertex_i as u32,
            flags: r.flags as u32,
            special: r.special as u32,
            tag: r.tag as u32,
            front_sidedef_i: r.front_sidedef_i as u32,
            back_sidedef_i: if r.back_sidedef_i != 0xFFFF {
                Some(r.back_sidedef_i as u32)
            } else {
                None
            },
        }
    }
}

impl From<RawLinedef> for Linedef {
    fn from(r: RawLinedef) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawLinedef {
    pub start_vertex_i: u16,
    pub end_vertex_i: u16,
    pub flags: u16,
    pub special: u16,
    pub tag: u16,
    pub front_sidedef_i: u16,
    pub back_sidedef_i: u16,
}

// SIDEDEF

#[derive(Clone, Copy, Debug)]
pub struct Sidedef {
    pub offset: IVec2,
    pub upper_texture: WadName,
    pub lower_texture: WadName,
    pub middle_texture: WadName,
    pub sector_i: u32,
}

impl Sidedef {
    pub fn from_raw(r: RawSidedef) -> Self {
        Self {
            offset: IVec2::new(r.x_offset as i32, r.y_offset as i32), // TODO: Check coords
            upper_texture: WadName::from_slice(&r.upper_texture),
            lower_texture: WadName::from_slice(&r.lower_texture),
            middle_texture: WadName::from_slice(&r.middle_texture),
            sector_i: r.sector_i as u32,
        }
    }
}

impl From<RawSidedef> for Sidedef {
    fn from(r: RawSidedef) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawSidedef {
    pub x_offset: i16,
    pub y_offset: i16,
    pub upper_texture: [u8; 8],
    pub lower_texture: [u8; 8],
    pub middle_texture: [u8; 8],
    pub sector_i: u16,
}

// VERTEX

#[derive(Clone, Copy, Debug)]
pub struct Vertex(pub IVec2);

impl Vertex {
    pub fn from_raw(r: RawVertex) -> Self {
        Self(from_doom_coords(r.x, r.y))
    }
}

impl From<RawVertex> for Vertex {
    fn from(r: RawVertex) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawVertex {
    pub x: i16,
    pub y: i16,
}

// SECTOR

#[derive(Clone, Copy, Debug)]
pub struct Sector {
    pub floor_height: i32,
    pub ceiling_height: i32,
    pub floor_texture: WadName,
    pub ceiling_texture: WadName,
    pub light_level: u8,
    pub special: u32,
    pub tag: u32,
}

impl Sector {
    pub fn from_raw(r: RawSector) -> Self {
        Self {
            floor_height: r.floor_height as i32,
            ceiling_height: r.ceiling_height as i32,
            floor_texture: WadName::from_slice(&r.floor_texture),
            ceiling_texture: WadName::from_slice(&r.ceiling_texture),
            light_level: r.light_level as u8,
            special: r.special as u32,
            tag: r.tag as u32,
        }
    }
}

impl From<RawSector> for Sector {
    fn from(r: RawSector) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawSector {
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_texture: [u8; 8],
    pub ceiling_texture: [u8; 8],
    pub light_level: u16,
    pub special: u16,
    pub tag: u16,
}

// FLAT

#[derive(Clone, Debug)]
pub struct Flat {
    pub data: [u8; 64 * 64],
}

impl Flat {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            data: bytes.try_into().context("Flat is not 64 by 64.")?,
        })
    }

    pub fn to_image(&self, palette: &DoomPalette) -> Image {
        let mut image_data = Vec::with_capacity(64 * 64 * 4);
        for pixel in &self.data {
            let color = palette.0[*pixel as usize];
            image_data.extend([color.r, color.g, color.b, 255]);
        }
        Image::new(
            bevy::render::render_resource::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            image_data,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        )
    }
}

// PICTURE

#[derive(Clone, Debug)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub offset: IVec2,
    pub data: Vec<Option<u8>>,
}

impl Picture {
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let header_size = std::mem::size_of::<RawPictureHeader>();
        let header: &RawPictureHeader = bytemuck::try_from_bytes(&bytes[0..header_size])?;
        let mut picture = Picture {
            width: header.width as u32,
            height: header.height as u32,
            offset: IVec2::new(header.left_offset as i32, header.top_offset as i32),
            data: vec![None; header.width as usize * header.height as usize],
        };

        let column_offsets: &[UnalignedU32] = bytemuck::try_cast_slice(
            &bytes[header_size..(header_size + picture.width as usize * 4)],
        )?;

        for (column_i, column_offset) in column_offsets.into_iter().enumerate() {
            let mut post_offset = column_offset.0 as usize;

            loop {
                // Loop through posts
                let post_top_delta = bytes[post_offset];
                if post_top_delta == 255 {
                    break;
                }

                let post_data_length = bytes[post_offset + 1];
                let post_data_offset = post_offset + 3;
                let post_data =
                    &bytes[post_data_offset..post_data_offset + post_data_length as usize];
                for (post_i, pixel) in post_data.into_iter().enumerate() {
                    let image_index =
                        (post_top_delta as usize + post_i) * picture.width as usize + column_i;
                    picture.data[image_index] = Some(*pixel);
                }

                post_offset += 4 + post_data_length as usize; // 2 extra unused bytes
            }
        }

        Ok(picture)
    }

    pub fn to_image(&self, palette: &DoomPalette) -> Image {
        let mut image_data = Vec::with_capacity((self.width * self.height) as usize * 4);
        for pixel_option in &self.data {
            if let Some(pixel) = pixel_option {
                // Not transparent
                let color = palette.0[*pixel as usize];
                image_data.extend([color.r, color.g, color.b, 255]);
            } else {
                image_data.extend([0, 0, 0, 0]);
            }
        }
        Image::new(
            bevy::render::render_resource::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            image_data,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        )
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawPictureHeader {
    pub width: u16,
    pub height: u16,
    pub left_offset: i16,
    pub top_offset: i16,
}

// TEXTURE

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: WadName,
    pub width: u32,
    pub height: u32,
    pub data: Vec<Option<u8>>,
}

impl Texture {
    pub fn to_image(&self, palette: &DoomPalette) -> Image {
        let mut image_data = Vec::with_capacity((self.width * self.height) as usize * 4);
        for pixel_option in &self.data {
            if let Some(pixel) = pixel_option {
                // Not transparent
                let color = palette.0[*pixel as usize];
                image_data.extend([color.r, color.g, color.b, 255]);
            } else {
                image_data.extend([0, 0, 0, 0]);
            }
        }
        Image::new(
            bevy::render::render_resource::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            image_data,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        )
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawTextureHeader {
    pub name: [u8; 8],
    pub _masked: u32,
    pub width: u16,
    pub height: u16,
    pub _column_directory: u32,
    pub num_patches: u16,
}

// PATCH

#[derive(Clone, Copy, Debug)]
pub struct Patch {
    pub offset: IVec2,
    pub pname_i: u32,
}

impl Patch {
    pub fn from_raw(r: RawPatch) -> Self {
        Self {
            offset: IVec2::new(r.offset_x as i32, r.offset_y as i32),
            pname_i: r.patch as u32,
        }
    }
}

impl From<RawPatch> for Patch {
    fn from(r: RawPatch) -> Self {
        Self::from_raw(r)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct RawPatch {
    pub offset_x: i16,
    pub offset_y: i16,
    pub patch: u16,
    pub _step_dir: i16,
    pub _colormap: u16,
}

// HELPER FUNCTIONS

pub fn bytes_to_vec<T: From<R>, R: Pod>(bytes: &[u8]) -> anyhow::Result<Vec<T>> {
    Ok(bytemuck::try_cast_slice(bytes)?
        .iter()
        .map(|&r| T::from(r))
        .collect())
}

fn from_doom_coords(x: i16, y: i16) -> IVec2 {
    // Flip Y from north to south
    IVec2::new(x as i32, -y as i32)
}

fn from_doom_angle(angle: i16) -> f32 {
    // Rotate from 0 degrees = east to 0 degrees = north
    let realign = angle as f32 - 90.0;
    // Round to nearest 45 degrees
    let rounded = (realign / 45.0).round();
    rounded * std::f32::consts::FRAC_PI_4
}

#[repr(C, packed)]
#[derive(Clone, Copy, Zeroable, Pod, Debug)]
pub struct UnalignedU32(pub u32);

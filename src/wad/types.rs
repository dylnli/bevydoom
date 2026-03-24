use std::fmt;

use bevy::math::IVec2;
use bytemuck::{Pod, Zeroable};

// WAD TYPE

#[derive(Debug, Clone, Copy)]
pub enum WadType {
    IWad,
    PWad,
}

// WAD NAME

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Zeroable, Pod)]
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

    pub fn from_str(s: &str) -> Self {
        Self::from_slice(s.as_bytes())
    }

    pub fn as_str(&self) -> &str {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(8);
        std::str::from_utf8(&self.0[..end]).unwrap_or("badname")
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

impl From<&[u8]> for WadName {
    fn from(s: &[u8]) -> Self {
        Self::from_slice(s)
    }
}

impl From<&str> for WadName {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

// HEADER

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawHeader {
    pub magic: [u8; 4],
    pub num_lumps: i32,
    pub directory_offset: i32,
}

// LUMP ENTRY

pub struct LumpEntry {
    pub offset: usize,
    pub size: usize,
    pub name: WadName,
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawLumpEntry {
    pub offset: i32,
    pub size: i32,
    pub name: [u8; 8],
}

// THING

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawThing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub thing_type: u16,
    pub flags: u16,
}

// LINEDEF

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
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

#[derive(Debug, Clone, Copy)]
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
            lower_texture: WadName::from_slice(&r.upper_texture),
            middle_texture: WadName::from_slice(&r.upper_texture),
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawSidedef {
    pub x_offset: i16,
    pub y_offset: i16,
    pub upper_texture: [u8; 8],
    pub lower_texture: [u8; 8],
    pub middle_texture: [u8; 8],
    pub sector_i: u16,
}

// VERTEX

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawVertex {
    pub x: i16,
    pub y: i16,
}

// SECTOR

#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct RawSector {
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_texture: [u8; 8],
    pub ceiling_texture: [u8; 8],
    pub light_level: u16,
    pub special: u16,
    pub tag: u16,
}

// HELPER FUNCTIONS

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

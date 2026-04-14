pub mod types;

use crate::wad::types::*;

use std::{fs, ops::Range, path::Path};

use anyhow::{Context, bail};
use bevy::prelude::*;
use bytemuck::Pod;

#[derive(Resource)]
pub struct WadFile {
    bytes: Vec<u8>,
    wad_type: WadType,
    directory: Vec<LumpEntry>,
    f_start: usize,
    f_end: usize,
    s_start: usize,
    s_end: usize,
    // Unused according to doomwiki.org?
    p_start: usize,
    p_end: usize,
}

impl WadFile {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let header: &RawHeader = bytemuck::from_bytes(&bytes[0..12]);
        if header.magic != *b"IWAD" && header.magic != *b"PWAD" {
            bail!("File is not a WAD!");
        }
        let wad_type = if header.magic == *b"IWAD" {
            WadType::IWad
        } else {
            WadType::PWad
        };

        let directory_offset = header.directory_offset as usize;
        let directory_size = header.num_lumps as usize * std::mem::size_of::<RawLumpEntry>();
        let directory_bytes = &bytes[directory_offset..directory_offset + directory_size];
        let directory = Self::bytes_to_vec::<LumpEntry, RawLumpEntry>(directory_bytes);

        let mut out = Self {
            bytes,
            wad_type,
            directory,
            f_start: 0,
            f_end: 0,
            s_start: 0,
            s_end: 0,
            p_start: 0,
            p_end: 0,
        };

        out.f_start = out.find_lump(WadName::from_slice(b"F_START"))?;
        out.f_end = out.find_lump(WadName::from_slice(b"F_END"))?;
        out.s_start = out.find_lump(WadName::from_slice(b"S_START"))?;
        out.s_end = out.find_lump(WadName::from_slice(b"S_END"))?;
        out.p_start = out.find_lump(WadName::from_slice(b"P_START"))?;
        out.p_end = out.find_lump(WadName::from_slice(b"P_END"))?;

        info!("{}", out.f_end - out.f_start);

        Ok(out)
    }

    fn find_lump_range(&self, name: WadName, start: usize, end: usize) -> anyhow::Result<usize> {
        return self.directory
            .iter()
            .enumerate()
            .take(end)
            .skip(start)
            .rev()
            .find_map(|(i, entry)| if entry.name == name { Some(i) } else { None })
            .with_context(|| format!("Failed to find lump {}.", name));
    }

    pub fn find_lump(&self, name: WadName) -> anyhow::Result<usize> {
        self.find_lump_range(name, 0, self.directory.len())
    }

    pub fn find_flat(&self, name: WadName) -> anyhow::Result<usize> {
        self.find_lump_range(name, self.f_start, self.f_end)
    }

    pub fn find_sprite(&self, name: WadName) -> anyhow::Result<usize> {
        self.find_lump_range(name, self.s_start, self.s_end)
    }

    pub fn find_patch(&self, name: WadName) -> anyhow::Result<usize> {
        self.find_lump_range(name, self.p_start, self.p_end)
    }

    pub fn load_lump(&self, index: usize) -> &[u8] {
        let lump_entry = &self.directory[index];
        return &self.bytes[lump_entry.offset..lump_entry.offset + lump_entry.size];
    }

    pub fn bytes_to_vec<T: From<R>, R: Pod>(bytes: &[u8]) -> Vec<T> {
        bytemuck::cast_slice(bytes)
            .iter()
            .map(|&r| T::from(r))
            .collect()
    }
}

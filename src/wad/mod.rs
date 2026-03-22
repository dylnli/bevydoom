pub mod types;

use std::{fs, path::Path};

use anyhow::anyhow;
use bevy::prelude::*;
use bytemuck::Pod;

use crate::wad::types::*;

#[derive(Resource)]
pub struct WadFile {
    bytes: Vec<u8>,
    wad_type: WadType,
    directory: Vec<LumpEntry>,
}

impl WadFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let header: &RawHeader = bytemuck::from_bytes(&bytes[0..12]);
        if header.magic != *b"IWAD" && header.magic != *b"PWAD" {
            return Err(anyhow!("File is not a WAD!").into());
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

        Ok(Self {
            bytes,
            wad_type,
            directory,
        })
    }

    pub fn find_lump(&self, name: WadName) -> Option<usize> {
        return self
            .directory
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, entry)| if entry.name == name { Some(i) } else { None });
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

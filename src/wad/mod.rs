pub mod types;

use crate::wad::types::*;

use std::{fs, path::Path};

use anyhow::{Context, bail};
use bevy::prelude::*;

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
        let header: &RawWadHeader =
            bytemuck::try_from_bytes(&bytes[0..std::mem::size_of::<RawWadHeader>()])?;
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
        let directory = bytes_to_vec::<LumpEntry, RawLumpEntry>(directory_bytes)?;

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

        Ok(out)
    }

    fn find_lump_range(&self, name: WadName, start: usize, end: usize) -> anyhow::Result<usize> {
        return self
            .directory
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
        &self.bytes[lump_entry.offset..lump_entry.offset + lump_entry.size]
    }

    pub fn load_flat(&self, index: usize) -> anyhow::Result<Flat> {
        Flat::from_bytes(self.load_lump(index))
    }

    pub fn load_picture(&self, index: usize) -> anyhow::Result<Picture> {
        Picture::from_bytes(self.load_lump(index))
    }

    pub fn load_textures(&self) -> anyhow::Result<Vec<Texture>> {
        let pnames_lump = self.load_lump(self.find_lump(WadName::from_slice(b"PNAMES"))?);
        let num_names: u32 = bytemuck::try_pod_read_unaligned(&pnames_lump[0..4])?;
        let pnames: Vec<WadName> =
            bytes_to_vec::<WadName, [u8; 8]>(&pnames_lump[4..4 + num_names as usize * 8])?;

        let mut textures =
            self.load_texturex(self.find_lump(WadName::from_slice(b"TEXTURE1"))?, &pnames)?;
        if let Some(texture2_i) = self.find_lump(WadName::from_slice(b"TEXTURE2")).ok() {
            textures.extend(self.load_texturex(texture2_i, &pnames)?);
        }

        Ok(textures)
    }

    fn load_texturex(&self, index: usize, pnames: &[WadName]) -> anyhow::Result<Vec<Texture>> {
        let lump = self.load_lump(index);

        let num_textures: u32 = bytemuck::try_pod_read_unaligned(&lump[0..4])?;
        let texture_offsets: &[UnalignedU32] =
            bytemuck::try_cast_slice(&lump[4..4 + num_textures as usize * 4])?;

        let mut textures = Vec::with_capacity(num_textures as usize);
        for i in 0..num_textures as usize {
            let offset = texture_offsets[i].0 as usize;

            let header_size = std::mem::size_of::<RawTextureHeader>();
            let header: &RawTextureHeader =
                bytemuck::try_from_bytes(&lump[offset..offset + header_size])?;

            let mut texture = Texture {
                name: WadName::from_slice(&header.name),
                width: header.width as u32,
                height: header.height as u32,
                data: vec![None; header.width as usize * header.height as usize],
            };

            let end_offset = offset
                + header_size
                + std::mem::size_of::<RawPatch>() * header.num_patches as usize;
            let patches = bytes_to_vec::<Patch, RawPatch>(&lump[offset + header_size..end_offset])?;

            for patch in &patches {
                let patch_name = pnames[patch.pname_i as usize];
                let patch_lump = self.load_lump(self.find_patch(patch_name)?);
                let picture = Picture::from_bytes(patch_lump)?;

                if texture.name == WadName::from_slice(b"BIGDOOR1") {
                    println!("{} {} {} {} {}", patch_name, patch.offset, picture.width, picture.height, picture.offset);
                }

                for y in 0..picture.height {
                    for x in 0..picture.width {
                        let picture_i = (y * picture.width + x) as usize;
                        if let Some(pixel) = picture.data[picture_i] {
                            let texture_i = (y as i32 + patch.offset.y) * texture.width as i32
                                + (x as i32 + patch.offset.x);
                            if texture_i < 0 || texture_i as usize >= texture.data.len() {
                                continue;
                            }
                            if texture.name == WadName::from_slice(b"BIGDOOR1") && patch_name == WadName::from_slice(b"DOOR2_1") {
                                if texture.data[texture_i as usize].is_some() {
                                    // println!("overwriting");
                                }
                            }
                            texture.data[texture_i as usize] = Some(pixel);
                        }
                    }
                }
            }

            textures.push(texture);
        }

        Ok(textures)
    }
}

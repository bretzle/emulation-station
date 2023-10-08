use std::ptr::NonNull;

use crate::core::video::ppu::{COLOR_TRANSPARENT, Ppu};

impl Ppu {
    pub(super) fn decode_tile_row_4bpp(&mut self, tile_base: u32, tile_number: u32, palette_number: u32, y: u32, horizontal_flip: bool, vertical_flip: bool) -> [u16; 8] {
        let mut pixels = [0; 8];
        let row = if vertical_flip { y ^ 7 } else { y } % 8;
        let tile_addr = tile_base + (tile_number * 32) + (row * 4);
        let mut palette_indices = self.bg.read::<u32>(tile_addr);

        for x in 0..8 {
            let column = if horizontal_flip { x ^ 7 } else { x };
            let palette_index = palette_indices & 0xf;
            let palette_addr = (palette_number * 32) + (palette_index * 2);

            let color = if palette_index == 0 { COLOR_TRANSPARENT } else { read(&self.palette_ram, palette_addr & 0x3fff) };
            pixels[column] = color;
            palette_indices >>= 4;
        }

        pixels
    }

    pub(super) fn decode_tile_row_8bpp(&mut self, tile_base: u32, tile_number: u32, palette_number: u32, y: u32, horizontal_flip: bool, vertical_flip: bool, extended_palette_slot: u32) -> [u16; 8] {
        let mut pixels = [0; 8];
        let row = if vertical_flip { y ^ 7 } else { y } % 8;
        let tile_addr = tile_base + (tile_number * 64) + (row * 8);
        let mut palette_indices = self.bg.read::<u64>(tile_addr);

        for x in 0..8 {
            let column = if horizontal_flip { x ^ 7 } else { x };
            let palette_index = (palette_indices & 0xff) as u32;

            let color = if palette_index == 0 {
                COLOR_TRANSPARENT
            } else if self.dispcnt.bg_extended_palette() {
                self.bg_extended_palette.read(extended_palette_slot * 0x2000 + (palette_number * 0xff + palette_index) * 2)
            } else {
                read(&self.palette_ram, (palette_index * 2) & 0x3ff)
            };
            pixels[column] = color;
            palette_indices >>= 8;
        }

        pixels
    }
}

fn read<T: Copy>(ptr: &NonNull<[u8]>, offset: u32) -> T {
    unsafe {
        *ptr.as_ref().as_ptr().add(offset as usize).cast()
    }
}
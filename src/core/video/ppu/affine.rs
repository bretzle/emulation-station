use std::ptr::NonNull;

use crate::core::video::ppu::{COLOR_TRANSPARENT, Ppu};
use crate::util::bit;

const EXTENDED_DIMENSIONS: [[u32; 2]; 4] = [[128, 128], [256, 256], [512, 256], [512, 512]];

impl Ppu {
    pub(super) fn render_affine(&mut self, id: usize) {
        todo!()
    }

    pub(super) fn render_extended(&mut self, id: usize) {
        let bgcnt = self.bgcnt[id];
        if bgcnt.palette_8bpp() {
            let data_base = bgcnt.screen_base() * 16384;
            let [width, height] = EXTENDED_DIMENSIONS[bgcnt.size()];

            if bit::<2>(bgcnt.0 as u32) {
                // direct color bitmap
                self.affine_loop(id, width, height, |ppu, pixel, x, y| {
                    let data_addr = data_base + (y * width + x) * 2;
                    let color = ppu.bg.read::<u16>(data_addr);

                    ppu.bg_layers[id][pixel] = if (color >> 15) & 0x1 != 0 {
                        color
                    } else {
                        COLOR_TRANSPARENT
                    };
                });
            } else {
                // 256 color bitmap
                self.affine_loop(id, width, height, |ppu, pixel, x, y| {
                    let data_addr = data_base + (y * width) + x;
                    let palette_index = ppu.bg.read::<u8>(data_addr);

                    ppu.bg_layers[id][pixel] = if palette_index == 0 {
                        COLOR_TRANSPARENT
                    } else {
                        read(&ppu.palette_ram, (palette_index as u32 * 2) & 0x3ff)
                    };
                });
            }
        } else {
            // 16-bit bgmap entries
            let screen_base = (bgcnt.screen_base() * 2048) + (self.dispcnt.screen_base() * 65536);
            let character_base = (bgcnt.character_base() * 16384) + (self.dispcnt.character_base() * 65536);
            let size = 128 << bgcnt.size();

            self.affine_loop(id, size, size, |ppu, pixel, x, y| {
                let screen_addr: u32 = screen_base + ((y / 8) * (size / 8) + (x / 8)) * 2;
                let tile_info: u16 = ppu.bg.read::<u16>(screen_addr);
                let tile_number = (tile_info & 0x3ff) as u32;
                let horizontal_flip = (tile_info >> 10) & 0x1 != 0;
                let vertical_flip = (tile_info >> 11) & 0x1 != 0;
                let palette_number = ((tile_info >> 12) & 0xf) as u32;

                let row = if vertical_flip { y ^ 7 } else { y } % 8;
                let column = if horizontal_flip { x ^ 7 } else { x } % 8;
                let tile_addr: u32 = character_base + (tile_number * 64) + (row * 8) + column;
                let palette_index = ppu.bg.read::<u8>(tile_addr) as u32;

                ppu.bg_layers[id][pixel] = if palette_index == 0 {
                    COLOR_TRANSPARENT
                } else if ppu.dispcnt.bg_extended_palette() {
                    let extended_palette_addr: u32 = (id as u32 * 8192) + ((palette_number * 256) + palette_index) * 2;
                    ppu.bg_extended_palette.read::<u16>(extended_palette_addr)
                } else {
                    read::<u16>(&ppu.palette_ram, (palette_index * 2) & 0x3ff)
                };
            });
        }
    }

    fn affine_loop<F: FnMut(&mut Self, usize, u32, u32)>(&mut self, id: usize, width: u32, height: u32, mut f: F) {
        let mut copy_x = self.internal_x[id - 2] as u32;
        let mut copy_y = self.internal_y[id - 2] as u32;
        let mut mosaic_bg_horizontal_counter = 0;

        for pixel in 0..256 {
            let mut x = copy_x >> 8;
            let mut y = copy_y >> 8;

            // apply horizontal mosaic
            if self.bgcnt[id].mosaic() && self.mosaic.bg_width() != 0 {
                todo!()
            } else {
                copy_x += self.bgpa[id - 2] as u32;
                copy_y += self.bgpc[id - 2] as u32;
            }

            if self.bgcnt[id].wraparound_ext_palette_slot() {
                x &= width - 1;
                y &= height - 1;
            } else if x < 0 || x >= width || y < 0 || y >= height {
                self.bg_layers[id][pixel] = COLOR_TRANSPARENT;
                continue;
            }

            f(self, pixel, x, y)
        }
    }
}

fn read<T: Copy>(ptr: &NonNull<[u8]>, offset: u32) -> T {
    unsafe {
        *ptr.as_ref().as_ptr().add(offset as usize).cast()
    }
}
use crate::core::video::ppu::Ppu;
use crate::util::{bit, get_field};

const TEXT_DIMENSIONS: [[u32; 2]; 4] = [[256, 256], [512, 256], [256, 512], [512, 512]];

impl Ppu {
    pub(super) fn render_text(&mut self, id: usize, mut line: u16) {
        if self.bgcnt[id].mosaic() {
            line -= self.mosaic_bg_vertical_counter
        }

        let y = ((line + self.bgvofs[id]) % 512) as u32;
        let mut screen_base = (self.dispcnt.screen_base() * 65536) + (self.bgcnt[id].screen_base() * 2048) + ((y / 8) % 32) * 64;
        let character_base = (self.dispcnt.character_base() * 65536) + (self.bgcnt[id].character_base() * 16384);
        let extended_palette_slot = id | (self.bgcnt[id].wraparound_ext_palette_slot() as usize * 2);
        let screen_width = TEXT_DIMENSIONS[self.bgcnt[id].size()][0];
        let screen_height = TEXT_DIMENSIONS[self.bgcnt[id].size()][1];

        if y >= 256 && screen_height == 512 {
            if screen_width == 512 {
                screen_base += 4096;
            } else {
                screen_base += 2048;
            }
        }

        let mut pixels = [0; 8];
        for tile in (0..=256).step_by(8) {
            let x = ((tile + self.bghofs[id]) % 512) as u32;
            let mut screen_addr = screen_base + ((x / 8) % 32) * 2;

            if x >= 256 && screen_width == 512 {
                screen_addr += 2048;
            }

            let tile_info = self.bg.read::<u16>(screen_addr) as u32;
            let tile_number = get_field::<0, 10>(tile_info);
            let horizontal_flip = bit::<10>(tile_info);
            let vertical_flip = bit::<11>(tile_info);
            let palette_number = get_field::<12, 4>(tile_info);

            pixels = if self.bgcnt[id].palette_8bpp() {
                todo!()
            } else {
                self.decode_tile_row_4bpp(character_base, tile_number, palette_number, y, horizontal_flip, vertical_flip)
            };

            for j in 0..8 {
                let offset = tile as usize + j - ((x as usize) % 8);
                if offset < 0 || offset >= 256 {
                    continue;
                }

                self.bg_layers[id][offset] = pixels[j];
            }
        }

        if self.bgcnt[id].mosaic() && self.mosaic.bg_width() != 0 {
            let mut mosaic_bg_horizontal_counter = 0;

            for i in 0..256 {
                self.bg_layers[id][i] = self.bg_layers[id][i - mosaic_bg_horizontal_counter];
                if mosaic_bg_horizontal_counter == self.mosaic.bg_width() as usize {
                    mosaic_bg_horizontal_counter = 0;
                } else {
                    mosaic_bg_horizontal_counter += 1;
                }
            }
        }
    }
}
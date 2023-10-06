use std::ptr::NonNull;

use crate::core::video::ppu::{COLOR_TRANSPARENT, Ppu, rgb555_to_rgb666, SpecialEffect};
use crate::util::get_field;

impl Ppu {
    pub(super) fn compose_scanline(&mut self, line: u16) {
        for x in 0..256 {
            // todo: check if a semi transparent object can override this logic
            if self.bldcnt.special_effect() != SpecialEffect::None {
                self.compose_pixel_with_special_effects(x, line)
            } else {
                self.compose_pixel(x, line)
            }
        }
    }

    fn compose_pixel_with_special_effects(&mut self, x: u16, line: u16) {
        let enabled = self.calculate_enabled_layers(x, line);
        let backdrop = read::<u16>(&self.palette_ram, 0);
        let mut targets = [5; 2];
        let mut priorities = [4; 2];

        // find the 2 top-most background pixels
        for i in (0..=3).rev() {
            let bg_pixel = self.bg_layers[i][x as usize];
            if ((enabled >> i) & 0x1 != 0) && bg_pixel != COLOR_TRANSPARENT {
                if self.bgcnt[i].priority() <= priorities[0] {
                    targets[1] = targets[0];
                    priorities[1] = priorities[0];
                    targets[0] = i;
                    priorities[0] = self.bgcnt[i].priority();
                } else if self.bgcnt[i].priority() <= priorities[1] {
                    targets[1] = i;
                    priorities[1] = self.bgcnt[i].priority();
                }
            }
        }

        // check if an object pixel can replace one of the background pixels
        // TODO: handle object window later
        if self.dispcnt.enable_obj() && self.obj_buffer[x as usize].color != COLOR_TRANSPARENT {
            if self.obj_buffer[x as usize].priority <= priorities[0] {
                targets[1] = targets[0];
                targets[0] = 4;
            } else if self.obj_buffer[x as usize].priority <= priorities[1] {
                targets[1] = 4;
            }
        }

        // map target indices to pixels
        // blending operations use 18-bit colours, so convert to that first
        let pixels: [u32; 2] = std::array::from_fn(|i| {
            match targets[i] {
                0 | 1 | 2 | 3 => self.bg_layers[targets[i]][x as usize] as u32,
                4 => self.obj_buffer[x as usize].color as u32,
                5 => backdrop as u32,
                _ => unreachable!()
            }
        }).map(rgb555_to_rgb666);

        let top_selected = (self.bldcnt.first_target() >> targets[0]) & 0x1 != 0;
        let bottom_selected = (self.bldcnt.second_target() >> targets[1]) & 0x1 != 0;

        // skip blending if the targets aren't selected
        if !top_selected || (self.bldcnt.special_effect() == SpecialEffect::AlphaBlending && !bottom_selected) {
            self.plot(x, line, pixels[0]);
            return;
        }

        self.plot(x, line, self.blend(pixels[0], pixels[1], self.bldcnt.special_effect()));
    }

    fn compose_pixel(&mut self, x: u16, line: u16) {
        let enabled = self.calculate_enabled_layers(x, line);
        let backdrop = read::<u16>(&self.palette_ram, 0);
        let mut pixel: u16 = backdrop;
        let mut priority = 4;

        for i in (0..=3).rev() {
            if ((enabled >> i) & 0x1 != 0) && self.bgcnt[i].priority() <= priority {
                let bg_pixel: u16 = self.bg_layers[i][x as usize];
                if bg_pixel != COLOR_TRANSPARENT {
                    pixel = bg_pixel;
                    priority = self.bgcnt[i].priority();
                }
            }
        }

        if self.dispcnt.enable_obj() && self.obj_buffer[x as usize].color != COLOR_TRANSPARENT {
            if self.obj_buffer[x as usize].priority <= priority as u32 {
                pixel = self.obj_buffer[x as usize].color;
            }
        }

        self.plot(x, line, rgb555_to_rgb666(pixel as u32))
    }

    fn calculate_enabled_layers(&self, x: u16, line: u16) -> u8 {
        let mut enabled = get_field::<8, 5>(self.dispcnt.0) as u8;
        let window = get_field::<13, 3>(self.dispcnt.0) as u8;

        if window != 0 {
            let win0_x1 = self.winh[0] >> 8;
            let win0_x2 = self.winh[0] & 0xff;
            let win0_y1 = self.winv[0] >> 8;
            let win0_y2 = self.winv[0] & 0xff;
            let win1_x1 = self.winh[1] >> 8;
            let win1_x2 = self.winh[1] & 0xff;
            let win1_y1 = self.winv[1] >> 8;
            let win1_y2 = self.winv[1] & 0xff;

            if self.dispcnt.enable_win0() && in_window_bounds(x, win0_x1, win0_x2) && in_window_bounds(line, win0_y1, win0_y2) {
                enabled &= (self.winin & 0xf) as u8;
            } else if self.dispcnt.enable_win1() && in_window_bounds(x, win1_x1, win1_x2) && in_window_bounds(line, win1_y1, win1_y2) {
                enabled &= ((self.winin >> 8) & 0xf) as u8;
            } else if self.dispcnt.enable_objwin() {
                todo!("PPU: handle object window");
            } else {
                enabled &= (self.winout & 0xf) as u8;
            }
        }

        enabled
    }

    fn blend(&self, top: u32, bottom: u32, effect: SpecialEffect) -> u32 {
        todo!()
    }
}

fn read<T: Copy>(ptr: &NonNull<[u8]>, offset: usize) -> T {
    unsafe {
        *ptr.as_ref().as_ptr().add(offset).cast()
    }
}

const fn in_window_bounds(coord: u16, start: u16, end: u16) -> bool {
    if start <= end {
        coord >= start && coord < end
    } else {
        coord >= start || coord < end
    }
}
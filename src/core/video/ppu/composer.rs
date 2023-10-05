use crate::core::video::ppu::{COLOR_TRANSPARENT, Ppu, rgb555_to_rgb666, SpecialEffect};

impl Ppu {
    pub(super) fn compose_scanline(&mut self, line: u16) {
        for x in 0..256 {
            // todo: check if a semi transparent object can override this logic
            if self.bldcnt.special_effect() != SpecialEffect::None {
                todo!()
            } else {
                self.compose_pixel(x, line)
            }
        }
    }

    fn compose_pixel(&mut self, x: u16, line: u16) {
        let enabled: u8 = todo!();
        let backdrop: u16 = todo!();
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
}
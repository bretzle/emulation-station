use std::mem::transmute;

use log::error;

use crate::core::video::ppu::Ppu;
use crate::util::{bit, get_field};

const OBJECT_DIMENSIONS: [[[u32; 2]; 4]; 4] = [[[8, 8], [16, 16], [32, 32], [64, 64]], [[16, 8], [32, 8], [32, 16], [64, 32]], [[8, 16], [8, 32], [16, 32], [32, 64]], [[0, 0], [0, 0], [0, 0], [0, 0]]];

#[repr(u32)]
#[derive(Copy, Clone, PartialEq)]
enum ObjectMode {
    Normal = 0,
    SemiTransparent = 1,
    ObjectWindow = 2,
    Bitmap = 3,
}

impl From<u32> for ObjectMode {
    fn from(value: u32) -> Self {
        unsafe { transmute(value) }
    }
}

impl Ppu {
    pub(super) fn render_objects(&mut self, line: u16) {
        let oam = unsafe { self.oam.as_ref() };
        for i in 0..128 {
            if (oam[(i * 8) + 1] & 0x3) == 0x2 {
                continue;
            }

            // todo: remove the casts
            let attributes = [
                read::<u16>(oam, i * 8) as u32,
                read::<u16>(oam, (i * 8) + 2) as u32,
                read::<u16>(oam, (i * 8) + 4) as u32,
            ];
            let mut affine_parameters = [0i16; 4];

            let mut y = get_field::<0, 8>(attributes[0]);
            let affine = bit::<8>(attributes[0]);
            let mode: ObjectMode = (get_field::<10, 2>(attributes[0])).into();
            let mosaic = bit::<12>(attributes[0]);
            let is_8bpp = bit::<13>(attributes[0]);
            let shape = get_field::<14, 2>(attributes[0]);
            let mut x = get_field::<0, 9>(attributes[1]);
            let horizontal_flip = !affine & bit::<12>(attributes[1]);
            let vertical_flip = !affine & bit::<13>(attributes[1]);
            let size = get_field::<14, 2>(attributes[1]);
            let tile_number = get_field::<0, 10>(attributes[2]);
            let priority = get_field::<10, 2>(attributes[2]);
            let palette_number = get_field::<12, 4>(attributes[2]);

            if x >= 256 {
                x -= 512;
            }
            if y >= 192 {
                y -= 256
            }

            let width = OBJECT_DIMENSIONS[shape as usize][size as usize][0] as i32;
            let height = OBJECT_DIMENSIONS[shape as usize][size as usize][1] as i32;
            let half_width = width / 2;
            let half_height = height / 2;

            x += half_width as u32;
            y += half_height as u32;

            if mosaic {
                error!("PPU: handle object mosaic");
            }

            if affine {
                todo!()
            } else {
                // for non-affine sprites, we can still use the general affine formula,
                // but instead use the parameters 0x100, 0, 0 and 0x100
                // these parameters get treated as identity values, meaning they don't
                // have any effect in the multiplication
                affine_parameters[0] = 0x100;
                affine_parameters[1] = 0;
                affine_parameters[2] = 0;
                affine_parameters[3] = 0x100;
            }

            if mode == ObjectMode::SemiTransparent {
                error!("PPU: handle semi transparent mode")
            }

            if mode == ObjectMode::ObjectWindow {
                error!("PPU: handle object window mode")
            }

            let local_y = line as i32 - y as i32;
            if local_y < -half_height || local_y >= half_height {
                continue;
            }

            for local_x in -half_width..=half_width {
                todo!()
            }
        }
    }
}

fn read<T: Copy>(ptr: &[u8], offset: usize) -> T {
    unsafe {
        *ptr.as_ptr().add(offset).cast()
    }
}
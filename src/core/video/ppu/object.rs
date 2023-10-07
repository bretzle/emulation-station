use std::mem::transmute;

use log::error;

use crate::core::video::ppu::{COLOR_TRANSPARENT, Ppu};
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

            let width = OBJECT_DIMENSIONS[shape as usize][size as usize][0];
            let height = OBJECT_DIMENSIONS[shape as usize][size as usize][1];
            let half_width = (width / 2) as i32;
            let half_height = (height / 2) as i32;

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
                let local_x = local_x as u32;
                let mut color = 0;
                let mut global_x = (x + local_x) as i32;
                if global_x < 0 || global_x >= 256 {
                    continue;
                }

                let mut transformed_x = ((((affine_parameters[0] as u32 * local_x) + (affine_parameters[1] as u32 * local_y as u32)) >> 8) + (width as u32 / 2));
                let mut transformed_y = ((((affine_parameters[2] as u32 * local_x) + (affine_parameters[3] as u32 * local_y as u32)) >> 8) + (height as u32 / 2));

                // make sure the transformed coordinates are still in bounds
                if transformed_x < 0 || transformed_y < 0 || transformed_x >= width || transformed_y >= height {
                    continue;
                }

                if horizontal_flip {
                    transformed_x = width - transformed_x - 1;
                }

                if vertical_flip {
                    transformed_y = height - transformed_y - 1;
                }

                let inner_tile_x = transformed_x % 8;
                let inner_tile_y = transformed_y % 8;
                let tile_x = transformed_x / 8;
                let tile_y = transformed_y / 8;
                let mut tile_addr = 0;

                if mode == ObjectMode::Bitmap {
                    todo!()
                } else if is_8bpp {
                    todo!()
                } else {
                    if self.dispcnt.tile_obj_mapping() {
                        tile_addr = (tile_number * (32 << self.dispcnt.tile_obj_1d_boundary())) + (tile_y * width * 4) as u32;
                    } else {
                        error!("PPU: handle 2d mapping 8bpp");
                    }

                    tile_addr += (tile_x * 32) as u32;
                    color = self.decode_obj_pixel_4bpp(tile_addr, palette_number, inner_tile_x, inner_tile_y);
                }

                let target_obj = &mut self.obj_buffer[global_x as usize];
                if color != COLOR_TRANSPARENT {
                    if priority < target_obj.priority {
                        target_obj.color = color;
                        target_obj.priority = priority;
                    }
                }
            }
        }
    }

    fn decode_obj_pixel_4bpp(&mut self, base: u32, number: u32, x: u32, y: u32) -> u16 {
        let indices = self.obj.read::<u8>(base + (y * 4) + (x / 2));
        let index = (indices >> (4 * (x & 0x1))) & 0xf;
        if index == 0 {
            COLOR_TRANSPARENT
        } else {
            unsafe { read(self.palette_ram.as_ref(), ((0x200 + (number * 32) + (index as u32 * 2)) & 0x3ff) as usize) }
        }
    }
}

fn read<T: Copy>(ptr: &[u8], offset: usize) -> T {
    unsafe {
        *ptr.as_ptr().add(offset).cast()
    }
}
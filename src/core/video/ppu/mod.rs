use crate::bitfield;
use crate::core::video::vram::VramRegion;
use crate::util::Shared;
use std::collections::HashSet;

const COLOR_TRANSPARENT: u16 = 0x8000;

bitfield! {
    struct DispCnt(u32) {
        bg_mode: u32 => 0 | 2,
        bg0_3d: bool => 3,
        tile_obj_mapping: bool => 4,
        bitmap_obj_2d: bool => 5,
        bitmap_obj_mapping: bool => 6,
        forced_blank: bool => 7,
        enable_bg0: bool => 8,
        enable_bg1: bool => 9,
        enable_bg2: bool => 10,
        enable_bg3: bool => 11,
        enable_obj: bool => 12,
        enable_win0: bool => 13,
        enable_win1: bool => 14,
        enable_objwin: bool => 15,
        display_mode: u32 => 16 | 17,
        vram_block: u32 => 18 | 19,
        tile_obj_1d_boundary: u32 => 20 | 21,
        bitmap_obj_1d_boundary: bool => 22,
        obj_during_hblank: bool => 23,
        character_base: u32 => 24 | 26,
        screen_base: u32 => 27 | 29,
        bg_extended_palette: bool => 30,
        obj_extended_palette: bool => 31
    }
}

bitfield! {
    struct Mosaic(u32) {
        bg_width: u16 => 0 | 3,
        bg_height: u16 => 4 | 7,
        obj_width: u16 => 8 | 11,
        obj_height: u16 => 12 | 15
    }
}

enum BrightnessMode {
    Disable = 0,
    Increase = 1,
    Decrease = 2,
    Reserved = 3,
}

bitfield! {
    struct MasterBright(u32) {
        factor: u32 => 0 | 4,
        // 5 | 13
        mode: u8 [BrightnessMode] => 14 | 15
    }
}

struct Object {
    priority: u32,
    color: u16,
}

pub struct Ppu {
    dispcnt: DispCnt,
    bgcnt: (),
    bghofs: (),
    bgvofs: (),
    bgpa: (),
    bgpb: (),
    bgpc: (),
    bgpd: (),
    bgx: [i32; 2],
    bgy: [i32; 2],
    internal_x: [i32; 2],
    internal_y: [i32; 2],
    winh: (),
    winv: (),
    winin: (),
    winout: (),
    mosaic: Mosaic,
    bldcnt: (),
    bldy: (),
    master_bright: MasterBright,
    bldalpha: (),

    mosaic_bg_vertical_counter: u16,

    framebuffer: Box<[u32; 256 * 192]>,
    converted_framebuffer: Box<[u32; 256 * 192]>,
    bg_layers: [[u16; 256]; 4],
    obj_buffer: [Object; 256],

    palette_ram: (),
    oam: (),
    bg: (),
    obj: (),
    bg_extended_palette: (),
    obj_extended_palette: (),
    lcdc: Shared<VramRegion>,
}

impl Ppu {
    pub fn new(
        bg: &Shared<VramRegion>,
        obj: &Shared<VramRegion>,
        bg_extended: &Shared<VramRegion>,
        obj_extended: &Shared<VramRegion>,
        lcdc: &Shared<VramRegion>,
    ) -> Self {
        Self {
            dispcnt: DispCnt(0),
            bgcnt: (),
            bghofs: (),
            bgvofs: (),
            bgpa: (),
            bgpb: (),
            bgpc: (),
            bgpd: (),
            bgx: [0; 2],
            bgy: [0; 2],
            internal_x: [0; 2],
            internal_y: [0; 2],
            winh: (),
            winv: (),
            winin: (),
            winout: (),
            mosaic: Mosaic(0),
            bldcnt: (),
            bldy: (),
            master_bright: MasterBright(0),
            bldalpha: (),
            mosaic_bg_vertical_counter: 0,
            framebuffer: Box::new([0; 256 * 192]),
            converted_framebuffer: Box::new([0; 256 * 192]),
            bg_layers: [[0; 256]; 4],
            obj_buffer: std::array::from_fn(|_| Object {
                priority: 0,
                color: 0,
            }),
            palette_ram: (),
            oam: (),
            bg: (),
            obj: (),
            bg_extended_palette: (),
            obj_extended_palette: (),
            lcdc: lcdc.clone(),
        }
    }

    pub fn reset(&mut self) {
        // todo

        self.reset_layers();
    }

    pub fn on_finish_frame(&mut self) {
        for i in 0..256 * 192 {
            self.converted_framebuffer[i] = rgb666_to_rgb888(self.framebuffer[i])
        }
    }

    pub fn fetch_framebuffer(&self) -> &[u32] {
        self.converted_framebuffer.as_slice()
    }

    pub fn render_scanline(&mut self, line: u16) {
        self.reset_layers();

        if line == 0 {
            self.internal_x = self.bgx;
            self.internal_y = self.bgy;
            self.mosaic_bg_vertical_counter = 0;
        }

        match self.dispcnt.display_mode() {
            0 => self.render_blank_screen(line),
            1 => todo!(),
            2 => self.render_vram_display(line),
            3 => todo!(),
            _ => unreachable!(),
        }

        self.apply_master_brightness(line);

        if self.mosaic_bg_vertical_counter == self.mosaic.bg_height() {
            self.mosaic_bg_vertical_counter = 0;
        } else {
            self.mosaic_bg_vertical_counter += 1;
        }

        if self.dispcnt.bg_mode() != 0 {
            todo!()
        }
    }

    fn reset_layers(&mut self) {
        for layer in &mut self.bg_layers {
            layer.fill(0)
        }

        for obj in &mut self.obj_buffer {
            obj.priority = 4;
            obj.color = COLOR_TRANSPARENT;
        }
    }

    fn apply_master_brightness(&mut self, line: u16) {
        let factor = self.master_bright.factor().min(16);
        if factor != 0 {
            todo!()
        }
    }

    fn render_blank_screen(&mut self, line: u16) {
        for x in 0..256 {
            self.plot(x, line, 0xffffffff)
        }
    }

    fn plot(&mut self, x: u16, y: u16, color: u32) {
        self.framebuffer[((256 * y) + x) as usize] = color;
    }

    pub fn write_dispcnt(&mut self, val: u32, mask: u32) {
        self.dispcnt.0 = (self.dispcnt.0 & !mask) | (val & mask)
    }

    fn render_vram_display(&mut self, line: u16) {
        for x in 0..256 {
            let addr = (self.dispcnt.vram_block() * 0x20000) + ((256 * line as u32) + x as u32) * 2;
            let data = self.lcdc.read::<u16>(addr) as u32;
            self.plot(x, line, rgb555_to_rgb666(data));
        }
    }
}

fn rgb555_to_rgb666(color: u32) -> u32 {
    let r = (color & 0x1f) * 2;
    let g = ((color >> 5) & 0x1f) * 2;
    let b = ((color >> 10) & 0x1f) * 2;
    (b << 12) | (g << 6) | r
}

fn rgb666_to_rgb888(colour: u32) -> u32 {
    let r = ((colour & 0x3f) * 255) / 63;
    let g = (((colour >> 6) & 0x3f) * 255) / 63;
    let b = (((colour >> 12) & 0x3f) * 255) / 63;
    // 0xff000000 | (r << 16) | (g << 8) | b
    (r << 16) | (g << 8) | b
}

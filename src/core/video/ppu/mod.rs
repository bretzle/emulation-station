use std::ptr::NonNull;
use log::error;

use crate::bitfield;
use crate::core::video::vram::VramRegion;
use crate::util::{set, Shared};

mod composer;
mod text;
mod tile_decoder;
mod object;

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
    #[derive(Clone, Copy)]
    struct BgCnt(u16) {
        priority: u32 => 0 | 1,
        character_base: u32 => 2 | 5,
        mosaic: bool => 6,
        palette_8bpp: bool => 7,
        screen_base: u32 => 8 | 12,
        wraparound_ext_palette_slot: bool => 13,
        size: usize => 14 | 15
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

#[derive(Clone, Copy, PartialEq)]
enum SpecialEffect {
    None = 0,
    AlphaBlending = 1,
    BrightnessIncrease = 2,
    BrightnessDecrease = 3,
}

bitfield! {
    #[derive(Clone, Copy)]
    struct BldCnt(u16) {
        first_target: u16 => 0 | 5,
        special_effect: u8 [SpecialEffect] => 6 | 7,
        second_target: u16 => 8 | 13
        // 14 | 15
    }
}

bitfield! {
    struct Bldy(u32) {
        evy: u32 => 0 | 4
        // 5 | 31
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

bitfield! {
    struct BldAlpha(u16) {
        eva: u16 => 0 | 4,
        // 5 | 7
        evb: u16 => 8 | 12
        // 13 | 15
    }
}

struct Object {
    priority: u32,
    color: u16,
}

pub struct Ppu {
    dispcnt: DispCnt,
    bgcnt: [BgCnt; 4],
    bghofs: [u16; 4],
    bgvofs: [u16; 4],
    bgpa: [i16; 2],
    bgpb: [i16; 2],
    bgpc: [i16; 2],
    bgpd: [i16; 2],
    bgx: [i32; 2],
    bgy: [i32; 2],
    internal_x: [i32; 2],
    internal_y: [i32; 2],
    winh: [u16; 2],
    winv: [u16; 2],
    winin: u16,
    winout: u16,
    mosaic: Mosaic,
    bldcnt: BldCnt,
    bldy: Bldy,
    master_bright: MasterBright,
    bldalpha: BldAlpha,

    mosaic_bg_vertical_counter: u16,

    framebuffer: Box<[u32; 256 * 192]>,
    converted_framebuffer: Box<[u8; 256 * 192 * 4]>,
    bg_layers: [[u16; 256]; 4],
    obj_buffer: [Object; 256],

    palette_ram: NonNull<[u8]>,
    oam: NonNull<[u8]>,
    bg: Shared<VramRegion>,
    obj: Shared<VramRegion>,
    bg_extended_palette: Shared<VramRegion>,
    obj_extended_palette: Shared<VramRegion>,
    lcdc: Shared<VramRegion>,
}

impl Ppu {
    pub fn new(
        bg: &Shared<VramRegion>,
        obj: &Shared<VramRegion>,
        bg_extended: &Shared<VramRegion>,
        obj_extended: &Shared<VramRegion>,
        lcdc: &Shared<VramRegion>,
        palette_ram: &mut [u8],
        oam: &mut [u8],
    ) -> Self {
        Self {
            dispcnt: DispCnt(0),
            bgcnt: [BgCnt(0); 4],
            bghofs: [0; 4],
            bgvofs: [0; 4],
            bgpa: [0; 2],
            bgpb: [0; 2],
            bgpc: [0; 2],
            bgpd: [0; 2],
            bgx: [0; 2],
            bgy: [0; 2],
            internal_x: [0; 2],
            internal_y: [0; 2],
            winh: [0; 2],
            winv: [0; 2],
            winin: 0,
            winout: 0,
            mosaic: Mosaic(0),
            bldcnt: BldCnt(0),
            bldy: Bldy(0),
            master_bright: MasterBright(0),
            bldalpha: BldAlpha(0),
            mosaic_bg_vertical_counter: 0,
            framebuffer: Box::new([0; 256 * 192]),
            converted_framebuffer: Box::new([0; 256 * 192 * 4]),
            bg_layers: [[0; 256]; 4],
            obj_buffer: std::array::from_fn(|_| Object { priority: 0, color: 0 }),
            palette_ram: NonNull::new(palette_ram).unwrap(),
            oam: NonNull::new(oam).unwrap(),
            bg: bg.clone(),
            obj: obj.clone(),
            bg_extended_palette: bg_extended.clone(),
            obj_extended_palette: obj_extended.clone(),
            lcdc: lcdc.clone(),
        }
    }

    pub fn reset(&mut self) {
        // todo

        self.reset_layers();
    }

    pub fn on_finish_frame(&mut self) {
        for i in 0..256 * 192 {
            let j = i * 4;
            self.converted_framebuffer[j..j + 4].copy_from_slice(&rgb666_to_rgb888(self.framebuffer[i]));
        }
    }

    pub fn fetch_framebuffer(&self) -> &[u8] {
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
            1 => self.render_graphics_display(line),
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

    fn apply_master_brightness(&mut self, _line: u16) {
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

    pub const fn read_dispcnt(&self) -> u32 {
        self.dispcnt.0
    }

    pub const fn read_bgcnt(&self, id: usize) -> u16 {
        self.bgcnt[id].0
    }

    pub const fn read_winin(&self) -> u16 {
        self.winin
    }

    pub const fn read_winout(&self) -> u16 {
        self.winout
    }

    pub fn write_dispcnt(&mut self, val: u32, mask: u32) {
        self.dispcnt.0 = (self.dispcnt.0 & !mask) | (val & mask)
    }

    pub fn write_bgcnt(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgcnt[id].0, val, mask)
    }
    pub fn write_bghofs(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bghofs[id], val, mask)
    }
    pub fn write_bgvofs(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgvofs[id], val, mask)
    }
    pub fn write_bgpa(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgpa[id], val as _, mask as _)
    }
    pub fn write_bgpb(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgpb[id], val as _, mask as _)
    }
    pub fn write_bgpc(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgpc[id], val as _, mask as _)
    }
    pub fn write_bgpd(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.bgpd[id], val as _, mask as _)
    }
    pub fn write_bgx(&mut self, id: usize, val: u32, mask: u32) {
        set(&mut self.bgx[id], val as _, (mask & 0xfffffff) as _);
        self.bgx[id] = (self.bgx[id] << 28 >> 28) as u32 as i32;
        self.internal_x[id] = self.bgx[id]
    }
    pub fn write_bgy(&mut self, id: usize, val: u32, mask: u32) {
        set(&mut self.bgy[id], val as _, (mask & 0xfffffff) as _);
        self.bgy[id] = (self.bgy[id] << 28 >> 28) as u32 as i32;
        self.internal_y[id] = self.bgy[id]
    }
    pub fn write_winh(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.winh[id], val, mask)
    }
    pub fn write_winv(&mut self, id: usize, val: u16, mask: u16) {
        set(&mut self.winv[id], val, mask)
    }
    pub fn write_winin(&mut self, val: u16, mask: u16) {
        set(&mut self.winin, val, mask)
    }
    pub fn write_winout(&mut self, val: u16, mask: u16) {
        set(&mut self.winout, val, mask)
    }
    pub fn write_mosaic(&mut self, val: u32, mask: u32) {
        set(&mut self.mosaic.0, val, mask & 0xffff)
    }
    pub fn write_bldcnt(&mut self, val: u16, mask: u16) {
        set(&mut self.bldcnt.0, val, mask)
    }
    pub fn write_bldalpha(&mut self, val: u16, mask: u16) {
        set(&mut self.bldalpha.0, val, mask)
    }
    pub fn write_bldy(&mut self, val: u32, mask: u32) {
        set(&mut self.bldy.0, val, mask)
    }
    pub fn write_master_bright(&mut self, val: u32, mask: u32) {
        set(&mut self.master_bright.0, val, mask)
    }

    fn render_vram_display(&mut self, line: u16) {
        for x in 0..256 {
            let addr = (self.dispcnt.vram_block() * 0x20000) + ((256 * line as u32) + x as u32) * 2;
            let data = self.lcdc.read::<u16>(addr) as u32;
            self.plot(x, line, rgb555_to_rgb666(data));
        }
    }

    fn render_graphics_display(&mut self, line: u16) {
        if self.dispcnt.enable_bg0() {
            if self.dispcnt.bg0_3d() || self.dispcnt.bg_mode() == 6 {
                error!("PPU: handle 3d rendering")
            } else {
                self.render_text(0, line)
            }
        }

        if self.dispcnt.enable_bg1() {
            if self.dispcnt.bg_mode() != 6 {
                self.render_text(1, line)
            }
        }

        if self.dispcnt.enable_bg2() {
            match self.dispcnt.bg_mode() {
                0 | 1 | 3 => self.render_text(2, line),
                2 | 4 => todo!("render_affine"),
                5 => todo!("render_extended"),
                6 => todo!("render_large"),
                _ => unreachable!(),
            }
        }

        if self.dispcnt.enable_bg3() {
            match self.dispcnt.bg_mode() {
                0 => self.render_text(3, line),
                1 | 2 => todo!("render_affine"),
                3 | 4 | 5 => todo!("render_extended"),
                _ => unreachable!(),
            }
        }

        if self.dispcnt.enable_obj() {
            self.render_objects(line)
        }

        self.compose_scanline(line);
    }
}

const fn rgb555_to_rgb666(color: u32) -> u32 {
    let r = (color & 0x1f) * 2;
    let g = ((color >> 5) & 0x1f) * 2;
    let b = ((color >> 10) & 0x1f) * 2;
    (b << 12) | (g << 6) | r
}

const fn rgb666_to_rgb888(colour: u32) -> [u8; 4] {
    let r = (((colour & 0x3f) * 255) / 63) as u8;
    let g = ((((colour >> 6) & 0x3f) * 255) / 63) as u8;
    let b = ((((colour >> 12) & 0x3f) * 255) / 63) as u8;
    // 0xff000000 | (r << 16) | (g << 8) | b
    // [0xff, r, g, b]
    // [b, g, r, 0xff]
    [r, g, b, 0xff]
}

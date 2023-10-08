use log::{debug, error};

use crate::bitfield;
use crate::core::hardware::dma::DmaTiming;
use crate::core::hardware::irq::IrqSource;
use crate::core::System;
use crate::util::{bit, get_field64, set, Shared};

bitfield! {
    #[derive(Clone, Copy)]
    struct AuxSpiCnt(u16) {
        baudrate: u16 => 0 | 1,
        // 2 | 5
        chipselect_hold: bool => 6,
        busy: bool => 7,
        // 8 | 12
        slot_mode: bool => 13,
        transfer_ready_irq: bool => 14,
        slot_enable: bool => 15
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    struct RomCtrl(u32) {
        key1_gap1_length: u32 => 0 | 12,
        key2_encrypt_data: bool => 13,
        // 14
        key2_apply_seed: bool => 15,
        key1_gap2_length: u32 => 16 | 21,
        key2_encrypt_command: bool => 22,
        word_ready: bool => 23,
        block_size: u32 => 24 | 26,
        transfer_rate: bool => 27,
        key1_gap_rate: bool => 28,
        resb_release_reset: bool => 29,
        data_direction: bool => 30,
        block_start: bool => 31
    }
}

enum CommandType {
    Dummy,
    ReadData,
    GetFirstId,
    GetSecondId,
    GetThirdId,
    ReadHeader,
    ReadSecureArea,
    None,
}

pub struct Cartridge {
    system: Shared<System>,
    file: Vec<u8>,
    header: Header,

    auxspicnt: AuxSpiCnt,
    auxspidata: u8,
    romctrl: RomCtrl,
    command_buffer: u64,
    command: u64,
    transfer_count: u32,
    transfer_size: u32,
    rom_position: u32,
    seed0: u64,
    seed1: u64,
    key1_encryption: bool,
    command_type: CommandType,
    key1_buffer: [u32; 0x412],
    key1_code: [u32; 3],
    secure_area: [u8; 0x4000],
    cartridge_inserted: bool,

    backup: (),
    backup_write_count: (),
}

impl Cartridge {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            file: vec![],
            header: Header::default(),
            auxspicnt: AuxSpiCnt(0),
            auxspidata: 0,
            romctrl: RomCtrl(0),
            command_buffer: 0,
            command: 0,
            transfer_count: 0,
            transfer_size: 0,
            rom_position: 0,
            seed0: 0,
            seed1: 0,
            key1_encryption: false,
            command_type: CommandType::Dummy,
            key1_buffer: [0; 0x412],
            key1_code: [0; 3],
            secure_area: [0; 0x4000],
            cartridge_inserted: false,

            backup: (),
            backup_write_count: (),
        }
    }

    pub fn load(&mut self, path: &str) {
        self.file = std::fs::read(path).unwrap();
        self.cartridge_inserted = true;
        self.header = Header::parse(&self.file);
        debug!("{:#?}", self.header);
    }

    pub fn direct_boot(&mut self) {
        // transfer the header + workaround for TinyFB
        for i in 0..0x170.min(self.file.len() as u32) {
            self.system.arm9.get_memory().write_byte(0x027ffe00 + i, self.file[i as usize])
        }

        // transfer the arm9 code
        for i in 0..self.header.arm9_size {
            self.system.arm9.get_memory().write_byte(self.header.arm9_ram_address + i, self.file[(self.header.arm9_offset + i) as usize])
        }

        // transfer the arm7 code
        for i in 0..self.header.arm7_size {
            self.system.arm7.get_memory().write_byte(self.header.arm7_ram_address + i, self.file[(self.header.arm7_offset + i) as usize])
        }

        debug!("Cartridge: cartridge data transferred into memory");
    }

    pub const fn get_arm9_entrypoint(&self) -> u32 {
        self.header.arm9_entrypoint
    }

    pub const fn get_arm7_entrypoint(&self) -> u32 {
        self.header.arm7_entrypoint
    }

    pub fn write_auxspicnt(&mut self, val: u16, mask: u16) {
        set(&mut self.auxspicnt.0, val, mask)
    }

    pub fn write_auxspidata(&mut self, val: u8) {
        if self.backup == () {
            return;
        }

        todo!()
    }

    pub fn write_romctrl(&mut self, val: u32, mask: u32) {
        let old = self.romctrl;
        set(&mut self.romctrl.0, val, mask);

        if !old.block_start() && self.romctrl.block_start() {
            self.start_transfer()
        }
    }

    pub fn write_command_buffer(&mut self, val: u64, mask: u64) {
        set(&mut self.command_buffer, val, mask)
    }

    pub const fn read_auxspicnt(&self) -> u16 {
        self.auxspicnt.0
    }

    pub const fn read_auxspidata(&self) -> u8 {
        self.auxspidata
    }

    pub const fn read_romctrl(&self) -> u32 {
        self.romctrl.0
    }

    pub fn read_data(&mut self) -> u32 {
        let mut data = 0xffffffff;
        if !self.romctrl.word_ready() {
            return data
        }

        if self.cartridge_inserted {
            match self.command_type {
                CommandType::Dummy => {}
                CommandType::ReadData => {
                    if self.rom_position < 0x8000 {
                        self.rom_position = 0x8000 + (self.rom_position & 0x1ff);
                    }

                    if (self.rom_position + self.transfer_count) >= self.file.len() as u32 {
                        error!("Cartridge: read data command exceeds rom size")
                    }

                    data = read::<u32>(&self.file, self.rom_position + self.transfer_count)
                }
                CommandType::GetFirstId | CommandType::GetSecondId | CommandType::GetThirdId => {
                    data = 0x1fc2
                }
                CommandType::ReadHeader => todo!(),
                CommandType::ReadSecureArea => todo!(),
                CommandType::None => unreachable!()
            }
        }

        self.transfer_count += 4;
        if self.transfer_count == self.transfer_size {
            self.romctrl.set_word_ready(false);
            self.romctrl.set_block_start(false);

            // todo: does this trigger on both cpus?
            if self.auxspicnt.transfer_ready_irq() {
                self.system.arm7.get_irq().raise(IrqSource::CartridgeTransfer);
                self.system.arm9.get_irq().raise(IrqSource::CartridgeTransfer);
            }
        } else {
            if bit::<11>(self.system.exmemcnt as u32) {
                self.system.dma7.trigger(DmaTiming::Slot1)
            } else {
                self.system.dma9.trigger(DmaTiming::Slot1)
            }
        }

        data
    }

    fn start_transfer(&mut self) {
        self.transfer_size = match self.romctrl.block_size() {
            0 => 0,
            7 => 4,
            other => 0x100 << other
        };

        self.command = self.command_buffer.swap_bytes();
        if self.key1_encryption {
            error!("Cartridge: handle key1 encryption")
        } else {
            self.process_decrypted_command()
        }

        if self.transfer_size == 0 {
            todo!()
        } else {
            self.transfer_count = 0;
            self.romctrl.set_word_ready(true);

            if bit::<11>(self.system.exmemcnt as u32) {
                self.system.dma7.trigger(DmaTiming::Slot1)
            } else {
                self.system.dma9.trigger(DmaTiming::Slot1)
            }
        }
    }

    fn process_decrypted_command(&mut self) {
        if !self.cartridge_inserted {
            return;
        }

        if (self.command & 0xff00000000ffffff) == 0xb700000000000000 {
            self.rom_position = get_field64::<24, 32>(self.command) as u32;
            self.command_type = CommandType::ReadData;
        } else if self.command == 0xb800000000000000 {
            self.command_type = CommandType::GetThirdId;
        } else if self.command == 0x9f00000000000000 {
            self.command_type = CommandType::Dummy;
        } else if self.command == 0x0000000000000000 {
            self.command_type = CommandType::ReadHeader;
        } else if self.command == 0x9000000000000000 {
            self.command_type = CommandType::GetFirstId;
        } else if (self.command >> 56) == 0x3c {
            self.key1_encryption = true;
            self.command_type = CommandType::None;
        } else {
            error!("Cartridge: handle decrypted command: {:016x}", self.command);
        }
    }
}

#[derive(Default, Debug)]
struct Header {
    title: String,
    arm9_offset: u32,
    // specifies from which offset in the rom data will be transferred to the arm9/arm7 bus
    arm9_entrypoint: u32,
    // specifies where r15 (program counter) will be set to in memory
    arm9_ram_address: u32,
    // specifies where in memory data from the cartridge will be transferred to
    arm9_size: u32,        // specifies the amount of bytes to be transferred from the cartridge to memory

    arm7_offset: u32,
    // specifies from which offset in the rom data will be transferred to the arm9/arm7 bus
    arm7_entrypoint: u32,
    // specifies where r15 (program counter) will be set to in memory
    arm7_ram_address: u32,
    // specifies where in memory data from the cartridge will be transferred to
    arm7_size: u32,        // specifies the amount of bytes to be transferred from the cartridge to memory

    icon_title_offset: u32, // specifies the offset in the rom image to where the icon and title is

    // used to identify the backup type
    gamecode: u32,
}

impl Header {
    fn parse(data: &[u8]) -> Self {
        macro_rules! read {
            ($t:ty, $start:literal) => {
                <$t>::from_le_bytes(data[$start..$start + std::mem::size_of::<$t>()].try_into().unwrap())
            };
        }

        Self {
            title: String::from_utf8_lossy(&data[0..12]).to_string(),
            arm9_offset: read!(u32, 0x20),
            arm9_entrypoint: read!(u32, 0x24),
            arm9_ram_address: read!(u32, 0x28),
            arm9_size: read!(u32, 0x2c),
            arm7_offset: read!(u32, 0x30),
            arm7_entrypoint: read!(u32, 0x34),
            arm7_ram_address: read!(u32, 0x38),
            arm7_size: read!(u32, 0x3c),
            icon_title_offset: read!(u32, 0x68),
            gamecode: read!(u32, 0x0c),
        }
    }
}

const fn read<T: Copy>(data: &[u8], offset: u32) -> T {
    unsafe {
        *data.as_ptr().add(offset as usize).cast()
    }
}
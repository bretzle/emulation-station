use crate::arm::memory::Memory;
use crate::core::System;
use crate::util::Shared;
use log::debug;

pub struct Cartridge {
    system: Shared<System>,
    file: Vec<u8>,
    header: Header,
    cartridge_inserted: bool,
}

impl Cartridge {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            file: vec![],
            header: Header::default(),
            cartridge_inserted: false,
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
            self.system
                .arm9
                .get_memory()
                .write_byte(0x027ffe00 + i, self.file[i as usize])
        }

        // transfer the arm9 code
        for i in 0..self.header.arm9_size {
            self.system.arm9.get_memory().write_byte(
                self.header.arm9_ram_address + i,
                self.file[(self.header.arm9_offset + i) as usize],
            )
        }

        // transfer the arm7 code
        // todo

        debug!("Cartridge: cartridge data transferred into memory");
    }

    pub const fn get_arm9_entrypoint(&self) -> u32 {
        self.header.arm9_entrypoint
    }
}

#[derive(Default, Debug)]
struct Header {
    title: String,
    arm9_offset: u32, // specifies from which offset in the rom data will be transferred to the arm9/arm7 bus
    arm9_entrypoint: u32, // specifies where r15 (program counter) will be set to in memory
    arm9_ram_address: u32, // specifies where in memory data from the cartridge will be transferred to
    arm9_size: u32, // specifies the amount of bytes to be transferred from the cartridge to memory

    arm7_offset: u32, // specifies from which offset in the rom data will be transferred to the arm9/arm7 bus
    arm7_entrypoint: u32, // specifies where r15 (program counter) will be set to in memory
    arm7_ram_address: u32, // specifies where in memory data from the cartridge will be transferred to
    arm7_size: u32, // specifies the amount of bytes to be transferred from the cartridge to memory

    icon_title_offset: u32, // specifies the offset in the rom image to where the icon and title is

    // used to identify the backup type
    gamecode: u32,
}

impl Header {
    fn parse(data: &[u8]) -> Self {
        macro_rules! read {
            ($t:ty, $start:literal) => {
                <$t>::from_le_bytes(
                    data[$start..$start + std::mem::size_of::<$t>()]
                        .try_into()
                        .unwrap(),
                )
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

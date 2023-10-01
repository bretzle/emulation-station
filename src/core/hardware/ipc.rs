use log::info;
use crate::arm::cpu::Arch;
use crate::bitfield;
use crate::core::hardware::irq::IrqSource;
use crate::core::System;
use crate::util::RingBuffer;
use crate::util::Shared;

bitfield! {
    #[derive(Clone, Copy, Default)]
    struct IpcSync(u32) {
        input: u8 => 0 | 3,
        // 4 | 7
        output: u8 => 8 | 11,
        // 12
        send_irq: bool => 13,
        enable_irq: bool => 14
    }
}

bitfield! {
    #[derive(Clone, Copy)]
    struct IpcFifoCnt(u16) {
        send_fifo_empty: bool => 0,
        send_fifo_full: bool => 1,
        send_fifo_empty_irq: bool => 2,
        send_fifo_clear: bool => 3,
        // 4 | 7
        receive_fifo_empty: bool => 8,
        receive_fifo_full: bool => 9,
        receive_fifo_empty_irq: bool => 10,
        // 11 | 13
        error: bool => 14,
        enable_fifos: bool => 15
    }
}

pub struct Ipc {
    system: Shared<System>,
    ipcsync: [IpcSync; 2],
    ipcfifocnt: [IpcFifoCnt; 2],
    fifo: [RingBuffer<u32, 16>; 2],
    ipcfiforecv: [u32; 2],
}

impl Ipc {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            ipcsync: Default::default(),
            ipcfifocnt: [IpcFifoCnt(0x101); 2],
            fifo: Default::default(),
            ipcfiforecv: Default::default(),
        }
    }

    pub fn reset(&mut self) {
        todo!()
    }

    pub fn read_ipcsync(&mut self, arch: Arch) -> u32 {
        self.ipcsync[arch as usize].0
    }
    pub fn read_ipcfifocnt(&mut self, arch: Arch) -> u16 {
        self.ipcfifocnt[arch as usize].0
    }
    pub fn read_ipcfiforecv(&mut self, arch: Arch) -> u32 {
        let tx = arch as usize;
        let rx = !arch as usize;

        if !self.fifo[rx].is_empty() {
            self.ipcfiforecv[tx] = self.fifo[rx].front();

            if self.ipcfifocnt[tx].enable_fifos() {
                self.fifo[rx].pop();

                if self.fifo[rx].is_empty() {
                    self.ipcfifocnt[rx].set_send_fifo_empty(true);
                    self.ipcfifocnt[tx].set_receive_fifo_empty(true);

                    if self.ipcfifocnt[rx].send_fifo_empty_irq() {
                        match arch {
                            Arch::ARMv4 => {
                                self.system.arm9.get_irq().raise(IrqSource::IPCSendEmpty)
                            }
                            Arch::ARMv5 => self.system.arm7.get_irq().raise(IrqSource::IPCSendEmpty)
                        }
                    }
                } else if self.fifo[rx].len() == 15 {
                    self.ipcfifocnt[rx].set_send_fifo_full(false);
                    self.ipcfifocnt[tx].set_receive_fifo_full(false);
                }
            }
        } else {
            self.ipcfifocnt[tx].set_error(true);
        }

        self.ipcfiforecv[tx]
    }

    pub fn write_ipcsync(&mut self, arch: Arch, val: u32, mut mask: u32) {
        let tx = arch as usize;
        let rx = !arch as usize;

        mask &= 0x6f00;
        self.ipcsync[tx].0 = (self.ipcsync[tx].0 & !mask) | (val & mask);
        self.ipcsync[rx].set_input(self.ipcsync[tx].output());

        if self.ipcsync[tx].send_irq() && self.ipcsync[rx].enable_irq() {
            match arch {
                Arch::ARMv4 => self.system.arm9.get_irq().raise(IrqSource::IPCSync),
                Arch::ARMv5 => self.system.arm7.get_irq().raise(IrqSource::IPCSync),
            }
        }
    }
    pub fn write_ipcfifocnt(&mut self, arch: Arch, val: u16, mut mask: u16) {
        let tx = arch as usize;
        let rx = !arch as usize;
        let send_fifo_empty_irq_old = self.ipcfifocnt[tx].send_fifo_empty_irq();
        let receive_fifo_empty_irq_old = self.ipcfifocnt[tx].receive_fifo_empty_irq();

        mask &= 0x8404;
        self.ipcfifocnt[tx].0 = (self.ipcfifocnt[tx].0 & !mask) | (val & mask);

        if val & (1 << 3) != 0 {
            self.fifo[tx].clear();
            self.ipcfifocnt[tx].set_send_fifo_empty(true);
            self.ipcfifocnt[tx].set_send_fifo_full(false);
            self.ipcfifocnt[rx].set_receive_fifo_empty(true);
            self.ipcfifocnt[rx].set_receive_fifo_full(false);

            if self.ipcfifocnt[tx].send_fifo_empty_irq() {
                match arch {
                    Arch::ARMv4 => self.system.arm7.get_irq().raise(IrqSource::IPCSendEmpty),
                    Arch::ARMv5 => self.system.arm9.get_irq().raise(IrqSource::IPCSendEmpty),
                }
            }
        }

        if !send_fifo_empty_irq_old
            && self.ipcfifocnt[tx].send_fifo_empty_irq()
            && self.ipcfifocnt[tx].send_fifo_empty()
        {
            match arch {
                Arch::ARMv4 => self.system.arm7.get_irq().raise(IrqSource::IPCSendEmpty),
                Arch::ARMv5 => self.system.arm9.get_irq().raise(IrqSource::IPCSendEmpty),
            }
        }

        if !receive_fifo_empty_irq_old
            && self.ipcfifocnt[tx].receive_fifo_empty_irq()
            && self.ipcfifocnt[tx].receive_fifo_empty()
        {
            match arch {
                Arch::ARMv4 => self
                    .system
                    .arm7
                    .get_irq()
                    .raise(IrqSource::IPCReceiveNonEmpty),
                Arch::ARMv5 => self
                    .system
                    .arm9
                    .get_irq()
                    .raise(IrqSource::IPCReceiveNonEmpty),
            }
        }

        if val & (1 << 14) != 0 {
            self.ipcfifocnt[tx].set_error(false);
        }
    }
    pub fn write_ipcfifosend(&mut self, arch: Arch, val: u32) {
        let tx = arch as usize;
        let rx = !arch as usize;

        if self.ipcfifocnt[tx].enable_fifos() {
            if self.fifo[tx].len() < 16 {
                self.fifo[tx].push(val);

                if self.fifo[tx].len() == 1 {
                    self.ipcfifocnt[tx].set_send_fifo_empty(false);
                    self.ipcfifocnt[rx].set_receive_fifo_empty(false);

                    if self.ipcfifocnt[rx].receive_fifo_empty_irq() {
                        match arch {
                            Arch::ARMv4 => self
                                .system
                                .arm9
                                .get_irq()
                                .raise(IrqSource::IPCReceiveNonEmpty),
                            Arch::ARMv5 => self
                                .system
                                .arm7
                                .get_irq()
                                .raise(IrqSource::IPCReceiveNonEmpty),
                        }
                    }
                } else if self.fifo[tx].len() == 16 {
                    self.ipcfifocnt[tx].set_send_fifo_full(true);
                    self.ipcfifocnt[rx].set_receive_fifo_full(true);
                }
            } else {
                self.ipcfifocnt[tx].set_error(true);
            }
        }
    }
}

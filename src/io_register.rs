use crate::bus::BusValue;

pub enum Interrupt {
    LcdVBlank,
    LcdHBlank,
    LcdVCounterMatch,
    Timer0Overflow,
    Timer1Overflow,
    Timer2Overflow,
    Timer3Overflow,
    SerialCommunication,
    Dma0,
    Dma1,
    Dma2,
    Dma3,
    Keypad,
    GamePak,
}

#[derive(Default)]
pub struct IORegister {
    // LCD registers
    pub disp_cnt: u16,
    pub green_swap: u16,
    pub disp_stat: u16,
    pub vcnt: u16,
    pub bg0_cnt: u16,
    pub bg1_cnt: u16,
    pub bg2_cnt: u16,
    pub bg3_cnt: u16,
    pub bg0_hofs: u16,
    pub bg0_vofs: u16,
    pub bg1_hofs: u16,
    pub bg1_vofs: u16,
    pub bg2_hofs: u16,
    pub bg2_vofs: u16,
    pub bg3_hofs: u16,
    pub bg3_vofs: u16,
    pub bg2_pa: u16,
    pub bg2_pb: u16,
    pub bg2_pc: u16,
    pub bg2_pd: u16,
    pub bg2_x: u32,
    pub bg2_y: u32,
    pub bg3_pa: u16,
    pub bg3_pb: u16,
    pub bg3_pc: u16,
    pub bg3_pd: u16,
    pub bg3_x: u32,
    pub bg3_y: u32,
    pub win0_h: u16,
    pub win1_h: u16,
    pub win0_v: u16,
    pub win1_v: u16,
    pub win_in: u16,
    pub win_out: u16,
    pub mosaic: u16,
    pub bld_cnt: u16,
    pub bld_alpha: u16,
    pub bld_y: u16,

    // Sound registers
    pub sound1_cnt_l: u16,
    pub sound1_cnt_h: u16,
    pub sound1_cnt_x: u16,
    pub sound2_cnt_l: u16,
    pub sound2_cnt_h: u16,
    pub sound3_cnt_l: u16,
    pub sound3_cnt_h: u16,
    pub sound3_cnt_x: u16,
    pub sound4_cnt_l: u16,
    pub sound4_cnt_h: u16,
    pub sound_cnt_l: u16,
    pub sound_cnt_h: u16,
    pub sound_cnt_x: u16,
    pub sound_bias: u16,
    pub wave_ram_a: Vec<u32>,
    pub wave_ram_b: Vec<u32>,
    pub fifo_a: u32,
    pub fifo_b: u32,

    // DMA registers
    pub dma0_sad: u32,
    pub dma0_dad: u32,
    pub dma0_cnt_l: u16,
    pub dma0_cnt_h: u16,
    pub dma1_sad: u32,
    pub dma1_dad: u32,
    pub dma1_cnt_l: u16,
    pub dma1_cnt_h: u16,
    pub dma2_sad: u32,
    pub dma2_dad: u32,
    pub dma2_cnt_l: u16,
    pub dma2_cnt_h: u16,
    pub dma3_sad: u32,
    pub dma3_dad: u32,
    pub dma3_cnt_l: u16,
    pub dma3_cnt_h: u16,

    // Timer registers
    pub tm0_cnt_l: u16,
    pub tm0_cnt_h: u16,
    pub tm1_cnt_l: u16,
    pub tm1_cnt_h: u16,
    pub tm2_cnt_l: u16,
    pub tm2_cnt_h: u16,
    pub tm3_cnt_l: u16,
    pub tm3_cnt_h: u16,

    // Serial communication 1 registers

    // Keypad registers
    pub key_input: u16,
    pub key_cnt: u16,

    // Serial communication 2 registers 

    // Interrupt, waitstate, power-down control registers
    pub int_e: u16,
    pub int_f: u16,
    pub wait_cnt: u16,
    pub ime: u16,
    pub postflg: u8,
    pub halt_cnt: u8,
    pub unknown: u8,
    pub memory_cnt: u32,
}

impl IORegister {
    pub fn new() -> IORegister {
        IORegister { 
            sound_bias: 0x200,
            wave_ram_a: vec![0; 4], 
            wave_ram_b: vec![0; 4],
            ..Default::default() 
        }
    }

    fn update_interrupt_ack(&mut self, val: u16) {
        self.int_f &= !val;
    }

    pub fn signal_interrupt(&mut self, int: Interrupt) -> bool {
        let mask = match int {
            Interrupt::LcdVBlank =>             0b0000000000001,
            Interrupt::LcdHBlank =>             0b0000000000010,
            Interrupt::LcdVCounterMatch =>      0b0000000000100,
            Interrupt::Timer0Overflow =>        0b0000000001000,
            Interrupt::Timer1Overflow =>        0b0000000010000,
            Interrupt::Timer2Overflow =>        0b0000000100000,
            Interrupt::Timer3Overflow =>        0b0000000100000,
            Interrupt::SerialCommunication =>   0b0000001000000,
            Interrupt::Dma0 =>                  0b0000010000000,
            Interrupt::Dma1 =>                  0b0000100000000,
            Interrupt::Dma2 =>                  0b0001000000000,
            Interrupt::Dma3 =>                  0b0010000000000,
            Interrupt::Keypad =>                0b0100000000000,
            Interrupt::GamePak =>               0b1000000000000,
        };

        if self.ime & 0x1 != 0 {
            if self.int_e & mask != 0 {
                self.int_f |= mask;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn read_register(&self, adr: u32) -> Option<BusValue> {
        println!("IO READ {:08x}", adr);

        match adr {
            0x000 => Some(self.disp_cnt.into()),
            0x002 => Some(self.green_swap.into()),
            0x004 => Some(self.disp_stat.into()),
            0x008 => Some(self.bg0_cnt.into()),
            0x00a => Some(self.bg1_cnt.into()),
            0x00c => Some(self.bg2_cnt.into()),
            0x00e => Some(self.bg3_cnt.into()),
            0x048 => Some(self.win_in.into()),
            0x04a => Some(self.win_out.into()),
            0x050 => Some(self.bld_cnt.into()),
            0x052 => Some(self.bld_alpha.into()),

            0x060 => Some(self.sound1_cnt_l.into()),
            0x062 => Some(self.sound1_cnt_h.into()),
            0x064 => Some(self.sound1_cnt_x.into()),
            0x068 => Some(self.sound2_cnt_l.into()),
            0x06c => Some(self.sound2_cnt_h.into()),
            0x070 => Some(self.sound3_cnt_l.into()),
            0x072 => Some(self.sound3_cnt_h.into()),
            0x074 => Some(self.sound3_cnt_x.into()),
            0x078 => Some(self.sound4_cnt_l.into()),
            0x07c => Some(self.sound4_cnt_h.into()),
            0x080 => Some(self.sound_cnt_l.into()),
            0x082 => Some(self.sound_cnt_h.into()),
            0x084 => Some(self.sound_cnt_x.into()),
            0x088 => Some(self.sound_bias.into()),
            0x090 => Some(self.wave_ram_a[0].into()),
            0x094 => Some(self.wave_ram_a[1].into()),
            0x098 => Some(self.wave_ram_a[2].into()),
            0x09c => Some(self.wave_ram_a[3].into()),

            0x0ba => Some(self.dma0_cnt_h.into()),
            0x0c6 => Some(self.dma1_cnt_h.into()),
            0x0d2 => Some(self.dma2_cnt_h.into()),
            0x0de => Some(self.dma3_cnt_h.into()),

            0x100 => Some(self.tm0_cnt_l.into()),
            0x102 => Some(self.tm0_cnt_h.into()),
            0x104 => Some(self.tm1_cnt_l.into()),
            0x106 => Some(self.tm1_cnt_h.into()),
            0x108 => Some(self.tm2_cnt_l.into()),
            0x10a => Some(self.tm2_cnt_h.into()),
            0x10c => Some(self.tm3_cnt_l.into()),
            0x10e => Some(self.tm3_cnt_h.into()),

            0x130 => Some(self.key_input.into()),
            0x132 => Some(self.key_cnt.into()),

            0x200 => Some(self.int_e.into()),
            0x202 => Some(self.int_f.into()),
            0x204 => Some(self.wait_cnt.into()),
            0x208 => Some(self.ime.into()),
            0x300 => Some(self.postflg.into()),
            0x410 => Some(self.unknown.into()),
            0x800 => Some(self.memory_cnt.into()),

            _ => Some(0u8.into()),
        }
    }

    pub fn write_register(&mut self, adr: u32, val: BusValue) -> Result<(), ()> {
        println!("IO WRITE {:08x} -> {:08x}", adr, std::convert::Into::<u32>::into(val));

        match adr {
            0x000 => {self.disp_cnt = val.into(); Ok(())},
            0x002 => {self.green_swap = val.into(); Ok(())},
            0x004 => {self.disp_stat = val.into(); Ok(())},
            0x008 => {self.bg0_cnt = val.into(); Ok(())},
            0x00a => {self.bg1_cnt = val.into(); Ok(())},
            0x00c => {self.bg2_cnt = val.into(); Ok(())},
            0x00e => {self.bg3_cnt = val.into(); Ok(())},
            0x010 => {self.bg0_hofs = val.into(); Ok(())},
            0x012 => {self.bg0_vofs = val.into(); Ok(())},
            0x014 => {self.bg1_hofs = val.into(); Ok(())},
            0x016 => {self.bg1_vofs = val.into(); Ok(())},
            0x018 => {self.bg2_hofs = val.into(); Ok(())},
            0x01a => {self.bg2_vofs = val.into(); Ok(())},
            0x01c => {self.bg3_hofs = val.into(); Ok(())},
            0x01e => {self.bg3_vofs = val.into(); Ok(())},
            0x020 => {self.bg2_pa = val.into(); Ok(())},
            0x022 => {self.bg2_pb = val.into(); Ok(())},
            0x024 => {self.bg2_pc = val.into(); Ok(())},
            0x026 => {self.bg2_pd = val.into(); Ok(())},
            0x028 => {self.bg2_x = val.into(); Ok(())},
            0x02c => {self.bg2_y = val.into(); Ok(())},
            0x030 => {self.bg3_pa = val.into(); Ok(())},
            0x032 => {self.bg3_pb = val.into(); Ok(())},
            0x034 => {self.bg3_pc = val.into(); Ok(())},
            0x036 => {self.bg3_pd = val.into(); Ok(())},
            0x038 => {self.bg3_x = val.into(); Ok(())},
            0x03c => {self.bg3_y = val.into(); Ok(())},
            0x040 => {self.win0_h = val.into(); Ok(())},
            0x042 => {self.win1_h = val.into(); Ok(())},
            0x044 => {self.win0_v = val.into(); Ok(())},
            0x046 => {self.win1_v = val.into(); Ok(())},
            0x048 => {self.win_in = val.into(); Ok(())},
            0x04a => {self.win_out = val.into(); Ok(())},
            0x04c => {self.mosaic = val.into(); Ok(())},
            0x050 => {self.bld_cnt = val.into(); Ok(())},
            0x052 => {self.bld_alpha = val.into(); Ok(())},
            0x054 => {self.bld_y = val.into(); Ok(())},

            0x060 => {self.sound1_cnt_l = val.into(); Ok(())},
            0x062 => {self.sound1_cnt_h = val.into(); Ok(())},
            0x064 => {self.sound1_cnt_x = val.into(); Ok(())},
            0x068 => {self.sound2_cnt_l = val.into(); Ok(())},
            0x06c => {self.sound2_cnt_h = val.into(); Ok(())},
            0x070 => {self.sound3_cnt_l = val.into(); Ok(())},
            0x072 => {self.sound3_cnt_h = val.into(); Ok(())},
            0x074 => {self.sound3_cnt_x = val.into(); Ok(())},
            0x078 => {self.sound4_cnt_l = val.into(); Ok(())},
            0x07c => {self.sound4_cnt_h = val.into(); Ok(())},
            0x080 => {self.sound_cnt_l = val.into(); Ok(())},
            0x082 => {self.sound_cnt_h = val.into(); Ok(())},
            0x084 => {self.sound_cnt_x = val.into(); Ok(())},
            0x088 => {self.sound_bias = val.into(); Ok(())},
            0x090 => {self.wave_ram_a[0] = val.into(); Ok(())},
            0x094 => {self.wave_ram_a[1] = val.into(); Ok(())},
            0x098 => {self.wave_ram_a[2] = val.into(); Ok(())},
            0x09c => {self.wave_ram_a[3] = val.into(); Ok(())},
            0x0a0 => {self.fifo_a = val.into(); Ok(())},
            0x0a4 => {self.fifo_b = val.into(); Ok(())},

            0x0b0 => {self.dma0_sad = val.into(); Ok(())},
            0x0b4 => {self.dma0_dad = val.into(); Ok(())},
            0x0b8 => {self.dma0_cnt_l = val.into(); Ok(())},
            0x0ba => {self.dma0_cnt_h = val.into(); Ok(())},
            0x0bc => {self.dma1_sad = val.into(); Ok(())},
            0x0c0 => {self.dma1_dad = val.into(); Ok(())},
            0x0c4 => {self.dma1_cnt_l = val.into(); Ok(())},
            0x0c6 => {self.dma1_cnt_h = val.into(); Ok(())},
            0x0c8 => {self.dma2_sad = val.into(); Ok(())},
            0x0cc => {self.dma2_dad = val.into(); Ok(())},
            0x0d0 => {self.dma2_cnt_l = val.into(); Ok(())},
            0x0d2 => {self.dma2_cnt_h = val.into(); Ok(())},
            0x0d4 => {self.dma3_sad = val.into(); Ok(())},
            0x0d8 => {self.dma3_dad = val.into(); Ok(())},
            0x0dc => {self.dma3_cnt_l = val.into(); Ok(())},
            0x0de => {self.dma3_cnt_h = val.into(); Ok(())},

            0x100 => {self.tm0_cnt_l = val.into(); Ok(())},
            0x102 => {self.tm0_cnt_h = val.into(); Ok(())},
            0x104 => {self.tm1_cnt_l = val.into(); Ok(())},
            0x106 => {self.tm1_cnt_h = val.into(); Ok(())},
            0x108 => {self.tm2_cnt_l = val.into(); Ok(())},
            0x10a => {self.tm2_cnt_h = val.into(); Ok(())},
            0x10c => {self.tm3_cnt_l = val.into(); Ok(())},
            0x10e => {self.tm3_cnt_h = val.into(); Ok(())},

            0x132 => {self.key_cnt = val.into(); Ok(())},

            0x114 |
            0x118 |
            0x1e2 => {println!("IO {:08x} -> {}", adr, std::convert::Into::<u32>::into(val)); Ok(())},

            0x120 |
            0x122 |
            0x124 |
            0x126 |
            0x128 |
            0x12A |
            0x12C |
            0x134 |
            0x140 |
            0x150 |
            0x154 |
            0x158 => {println!("SERIAL IO {:08x} -> {}", adr, std::convert::Into::<u32>::into(val)); Ok(())},

            0x200 => {self.int_e = val.into(); Ok(())},
            0x202 => {self.update_interrupt_ack(val.into()); Ok(())},
            0x204 => {self.wait_cnt = val.into(); Ok(())},
            0x208 => {self.ime = val.into(); Ok(())},
            0x300 => {self.postflg = val.into(); Ok(())},
            0x301 => {self.halt_cnt = val.into(); Ok(())},
            0x410 => {self.unknown = val.into(); Ok(())},
            0x800 => {self.memory_cnt = val.into(); Ok(())},

            _ => {
                println!("Invalid IO access {:08x}", adr);
                Ok(())
            },
        }
    }
}
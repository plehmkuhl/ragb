use crate::emulator::*;

pub enum BusValue {
    Dword(u32),
    Word(u16)
}

impl EmulatorState {
    pub fn read_bus(&mut self, adr: u32) -> Option<BusValue> {
        match adr {
            // General internal memory
            0x00000000..=0x00003FFF => None,
            0x00004000..=0x01FFFFFF => None,
            0x02000000..=0x0203FFFF => Some(BusValue::Word(self.ewram[((adr - 0x02000000) / 2) as usize])),
            0x02040000..=0x02FFFFFF => None,
            0x03000000..=0x03007FFF => Some(BusValue::Dword(self.iwram[((adr - 0x03000000) / 4) as usize])),
            0x03008000..=0x03FFFFFF => None,
            0x04000000..=0x040003FE => Some(BusValue::Dword(self.io[((adr - 0x04000000) / 4) as usize])),
            0x04000400..=0x04FFFFFF => None,

            // Internal display memory
            0x05000000..=0x050003FF => Some(BusValue::Word(self.pram[((adr - 0x05000000) / 2) as usize])),
            0x05000400..=0x05FFFFFF => None,
            0x06000000..=0x06017FFF => Some(BusValue::Word(self.vram[((adr - 0x06000000) / 2) as usize])),
            0x06018000..=0x06FFFFFF => None,
            0x07000000..=0x070003FF => Some(BusValue::Dword(self.oam[((adr - 0x07000000) / 4) as usize])),
            0x07000400..=0x07FFFFFF => None,

            // External memory
            0x08000000..=0x09FFFFFF => None,
            0x0A000000..=0x0BFFFFFF => None,
            0x0C000000..=0x0DFFFFFF => None,
            0x0E000000..=0x0E00FFFF => None,
            0x0E010000..=0x0FFFFFFF => None,

            // Unused area
            _ => None
        }
    }

    pub fn write_bus(&mut self, adr: u32, val: BusValue) {
        match adr {
            // General internal memory
            0x00000000..=0x00003FFF => (),
            0x00004000..=0x01FFFFFF => (),
            0x02000000..=0x0203FFFF => self.ewram[((adr - 0x02000000) / 2) as usize] = match val {
                BusValue::Word(v) => v,
                BusValue::Dword(v) => (v & 0xFFFF) as u16,
            },
            0x02040000..=0x02FFFFFF => (),
            0x03000000..=0x03007FFF => self.iwram[((adr - 0x03000000) / 4) as usize] = match val {
                BusValue::Word(v) => v as u32,
                BusValue::Dword(v) => v,
            },
            0x03008000..=0x03FFFFFF => (),
            0x04000000..=0x040003FE => self.io[((adr - 0x04000000) / 4) as usize] = match val {
                BusValue::Word(v) => v as u32,
                BusValue::Dword(v) => v,
            },
            0x04000400..=0x04FFFFFF => (),

            // Internal display memory
            0x05000000..=0x050003FF => self.pram[((adr - 0x05000000) / 2) as usize] = match val {
                BusValue::Word(v) => v,
                BusValue::Dword(v) => (v & 0xFFFF) as u16,
            },
            0x05000400..=0x05FFFFFF => (),
            0x06000000..=0x06017FFF => self.vram[((adr - 0x06000000) / 2) as usize] = match val {
                BusValue::Word(v) => v,
                BusValue::Dword(v) => (v & 0xFFFF) as u16,
            },
            0x06018000..=0x06FFFFFF => (),
            0x07000000..=0x070003FF => self.oam[((adr - 0x07000000) / 4) as usize] = match val {
                BusValue::Word(v) => v as u32,
                BusValue::Dword(v) => v,
            },
            0x07000400..=0x07FFFFFF => (),

            // External memory
            0x08000000..=0x09FFFFFF => (),
            0x0A000000..=0x0BFFFFFF => (),
            0x0C000000..=0x0DFFFFFF => (),
            0x0E000000..=0x0E00FFFF => (),
            0x0E010000..=0x0FFFFFFF => (),

            // Unused area
            _ => ()
        }
    }
}
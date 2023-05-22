use crate::gba::*;

#[derive(Clone,Copy)]
pub enum BusValue {
    Word(u32),
    HalfWord(u16),
    Byte(u8),
}

impl Into<u8> for BusValue {
    fn into(self) -> u8 {
        match self {
            BusValue::Byte(b) => b,
            BusValue::HalfWord(h) => (h & 0xFF) as u8,
            BusValue::Word(w) => (w & 0xFF) as u8,
        }
    }
}

impl Into<u16> for BusValue {
    fn into(self) -> u16 {
        match self {
            BusValue::Byte(b) => b as u16,
            BusValue::HalfWord(h) => h,
            BusValue::Word(w) => (w & 0xFFFF) as u16,
        }
    }
}

impl Into<u32> for BusValue {
    fn into(self) -> u32 {
        match self {
            BusValue::Byte(b) => b as u32,
            BusValue::HalfWord(h) => h as u32,
            BusValue::Word(w) => w,
        }
    }
}

impl From<u8> for BusValue {
    fn from(val: u8) -> BusValue {
        BusValue::Byte(val)
    }
}

impl From<u16> for BusValue {
    fn from(val: u16) -> BusValue {
        BusValue::HalfWord(val)
    }
}

impl From<u32> for BusValue {
    fn from(val: u32) -> BusValue {
        BusValue::Word(val)
    }
}

#[derive(Clone,Copy)]
pub enum BusWidth {
    Word,
    HalfWord,
    Byte,
}

impl GbaSystem {
    pub fn read_bus(&mut self, adr: u32) -> Option<BusValue> {
        match adr {
            // General internal memory
            0x00000000..=0x00003FFF => Some(BusValue::Word(self.bios[(adr >> 2) as usize])),
            0x00004000..=0x01FFFFFF => None, // Reserved
            0x02000000..=0x02FFFFFF => Some(BusValue::HalfWord(self.ewram[((adr & 0x03FFFF) >> 2) as usize])),
            0x03000000..=0x03FFFFFF => Some(BusValue::Word(self.iwram[((adr & 0x07FFF) >> 2) as usize])),
            0x04000000..=0x04FFFFFF => self.io_register.read_register(adr & 0xFFFFFF), //Some(BusValue::Word(self.io[((adr & 0x7FF) >> 2) as usize])),

            // Internal display memory
            0x05000000..=0x050003FF => Some(BusValue::HalfWord(self.pram[((adr - 0x05000000) >> 1) as usize])),
            0x05000400..=0x05FFFFFF => None, // Reserved
            0x06000000..=0x06017FFF => Some(BusValue::HalfWord(self.vram[((adr - 0x06000000) >> 1) as usize])),
            0x06018000..=0x06FFFFFF => None, // Reserved
            0x07000000..=0x070003FF => Some(BusValue::Word(self.oam[((adr - 0x07000000) >> 2) as usize])),
            0x07000400..=0x07FFFFFF => None, // Reserved

            // External memory
            0x08000000..=0x09FFFFFF => Some(BusValue::HalfWord(self.pack[((adr - 0x08000000) >> 1) as usize])),
            0x0A000000..=0x0BFFFFFF => Some(BusValue::HalfWord(self.pack[((adr - 0x0A000000) >> 1) as usize])),
            0x0C000000..=0x0DFFFFFF => Some(BusValue::HalfWord(self.pack[((adr - 0x0C000000) >> 1) as usize])),
            0x0E000000..=0x0E00FFFF => Some(BusValue::Byte(self.sram[(adr - 0x0E000000) as usize])),
            0x0E010000..=0x0FFFFFFF => None, // Reserved

            // Unused area
            _ => None
        }
    }

    pub fn read_bus_byte(&mut self, adr: u32) -> Option<u8> {
        match self.read_bus(adr) {
            Some(BusValue::Byte(d)) => Some(d),
            Some(BusValue::HalfWord(d)) => {
                let b = d.to_le_bytes();
                Some(b[(adr & 0x1) as usize])
            },
            Some(BusValue::Word(d)) => {
                let b = d.to_le_bytes();
                Some(b[(adr & 0x3) as usize])
            },
            None => None,
        }
    }

    pub fn read_bus_half_word(&mut self, adr: u32) -> Option<u16> {
        match self.read_bus(adr) {
            Some(BusValue::Byte(d)) => {
                let mut b = [0;2];
                b[0] = d;
                b[1] = self.read_bus_byte(adr + 1)?;

                Some(u16::from_le_bytes(b))
            },
            Some(BusValue::HalfWord(d)) => Some(d),
            Some(BusValue::Word(d)) => {
                let b = d.to_le_bytes();
                let idx = ((adr & 0x3)) as usize;
                Some(u16::from_le_bytes(b[idx..idx+2].try_into().unwrap()))
            },
            None => None,
        }
    }

    pub fn read_bus_word(&mut self, adr: u32) -> Option<u32> {
        match self.read_bus(adr) {
            Some(BusValue::Byte(d)) => {
                let mut b = [0;4];
                b[0] = d;

                for n in 1..4 {
                    b[n as usize] = self.read_bus_byte(adr + n)?;
                }

                Some(u32::from_le_bytes(b))
            },
            Some(BusValue::HalfWord(d)) => {
                let b1 = d.to_le_bytes();
                let b2 = self.read_bus_half_word(adr + 2)?.to_le_bytes();
                
                Some(u32::from_le_bytes([b1, b2].concat().try_into().unwrap()))
            },
            Some(BusValue::Word(d)) => Some(d),
            None => None,
        }
    }

    pub fn write_bus(&mut self, adr: u32, write_val: BusValue) -> Result<(),()> {
        assert_eq!(adr & 0x1, 0, "Memory access unaligned!");

        match self.read_bus(adr) {
            Some(BusValue::Byte(_)) => match write_val {
                BusValue::Byte(_) => self.write_bus_raw(adr, write_val),
                BusValue::HalfWord(v) => {
                    let data = v.to_le_bytes();

                    for b in 0..data.len() {
                        self.write_bus_raw(adr+b as u32, BusValue::Byte(data[b]))?;
                    }

                    Ok(())
                }
                BusValue::Word(v) => {
                    let data = v.to_le_bytes();

                    for b in 0..data.len() {
                        self.write_bus_raw(adr+b as u32, BusValue::Byte(data[b]))?;
                    }

                    Ok(())
                },
            },
            Some(BusValue::HalfWord(v)) => match write_val {
                BusValue::Byte(write_val) => {
                    let mut data = v.to_le_bytes();
                    data[(adr & 0x1) as usize] = write_val;
                    self.write_bus_raw(adr, BusValue::HalfWord(u16::from_le_bytes(data)))?;
                    Ok(())
                },
                BusValue::HalfWord(_) => {
                    self.write_bus_raw(adr, write_val)?;
                    Ok(())
                }
                BusValue::Word(write_val) => {
                    let data = write_val.to_le_bytes();

                    self.write_bus_raw(adr, BusValue::HalfWord(u16::from_le_bytes(data[0..2].try_into().unwrap())))?;
                    self.write_bus_raw(adr, BusValue::HalfWord(u16::from_le_bytes(data[2..4].try_into().unwrap())))?;
                    Ok(())
                },
            },
            Some(BusValue::Word(v)) => match write_val {
                BusValue::Byte(write_val) => {
                    let mut data = v.to_le_bytes();
                    data[(adr & 0x3) as usize] = write_val;
                    self.write_bus_raw(adr, BusValue::Word(u32::from_le_bytes(data)))?;
                    Ok(())
                },
                BusValue::HalfWord(write_val) => {
                    let mut data = v.to_le_bytes();
                    let w_data = write_val.to_le_bytes();

                    data[(adr & 0x3) as usize] = w_data[0];
                    data[(adr & 0x3) as usize + 1] = w_data[1];

                    self.write_bus_raw(adr, BusValue::Word(u32::from_le_bytes(data)))?;

                    Ok(())
                }
                BusValue::Word(_) => {
                    self.write_bus_raw(adr, write_val)?;
                    Ok(())
                },
            },
            None => self.write_bus_raw(adr, write_val),
        }
    }

    pub fn write_bus_raw(&mut self, adr: u32, val: BusValue) -> Result<(),()> {
        match adr {
            // General internal memory
            0x00000000..=0x00003FFF => Err(()),
            0x00004000..=0x01FFFFFF => Err(()),
            0x02000000..=0x0203FFFF => {
                self.ewram[((adr - 0x02000000) >> 1) as usize] = match val {
                    BusValue::HalfWord(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x02040000..=0x02FFFFFF => Err(()),
            0x03000000..=0x03FFFFFF => {
                self.iwram[((adr & 0x07FFF) >> 2) as usize] = match val {
                    BusValue::Word(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x04000000..=0x04FFFFFF => self.io_register.write_register(adr & 0xFFFFFF, val),

            // Internal display memory
            0x05000000..=0x050003FF => {
                self.pram[((adr - 0x05000000) >> 1) as usize] = match val {
                    BusValue::HalfWord(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x05000400..=0x05FFFFFF => Err(()),
            0x06000000..=0x06017FFF => {
                self.vram[((adr - 0x06000000) >> 1) as usize] = match val {
                    BusValue::HalfWord(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x06018000..=0x06FFFFFF => Err(()),
            0x07000000..=0x070003FF => {
                self.oam[((adr - 0x07000000) >> 2) as usize] = match val {
                    BusValue::Word(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x07000400..=0x07FFFFFF => Err(()),

            // External memory
            0x08000000..=0x09FFFFFF => Err(()),
            0x0A000000..=0x0BFFFFFF => Err(()),
            0x0C000000..=0x0DFFFFFF => Err(()),
            0x0E000000..=0x0E00FFFF => {
                self.sram[(adr - 0x0E000000) as usize] = match val {
                    BusValue::Byte(v) => Ok(v),
                    _ => Err(()),
                }?;
                Ok(())
            },
            0x0E010000..=0x0FFFFFFF => Err(()),

            // Unused area
            _ => Err(())
        }
    }
}
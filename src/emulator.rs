pub struct EmulatorState {
    // Memory (32-Bit bus)
    pub iwram:[u32; 8192],
    pub oam:[u32; 256],
    pub io:[u32; 256],

    // Memory (16-Bit bus)
    pub ewram:[u16; 131072],
    pub vram:[u16; 49152],
    pub pram:[u16; 512],

}

impl EmulatorState {
    pub fn new() -> EmulatorState {
        EmulatorState {
            iwram: [0; 8192],
            oam: [0; 256],
            io: [0; 256],
            ewram: [0; 131072],
            vram: [0; 49152],
            pram: [0; 512],
        }
    }
}
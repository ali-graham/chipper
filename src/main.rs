fn main() {
    let mut _opcode: u16;

    let mut _v0: u8;
    let mut _v1: u8;
    let mut _v2: u8;
    let mut _v3: u8;
    let mut _v4: u8;
    let mut _v5: u8;
    let mut _v6: u8;
    let mut _v7: u8;
    let mut _v8: u8;
    let mut _v9: u8;
    let mut _va: u8;
    let mut _vb: u8;
    let mut _vc: u8;
    let mut _vd: u8;
    let mut _ve: u8;
    let mut _vf: u8; // carry flag

    let mut _i: u16;

    let mut _pc: u16;

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xFFF - Program ROM and work RAM
    let mut _memory: [u8; 4096];
    let mut _gfx: [u8; 64 * 32];

    let mut _delay_timer: u8;

    // The system’s buzzer sounds whenever the sound timer reaches zero.
    let mut _sound_timer: u8;
}

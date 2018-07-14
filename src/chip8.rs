const CHIP8_FONTSET: [u8; 80] = [
  0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
  0x20, 0x60, 0x20, 0x20, 0x70, // 1
  0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
  0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
  0x90, 0x90, 0xF0, 0x10, 0x10, // 4
  0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
  0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
  0xF0, 0x10, 0x20, 0x40, 0x40, // 7
  0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
  0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
  0xF0, 0x90, 0xF0, 0x90, 0x90, // A
  0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
  0xF0, 0x80, 0x80, 0x80, 0xF0, // C
  0xE0, 0x90, 0x90, 0x90, 0xE0, // D
  0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
  0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    pub v0: u8,
    pub v1: u8,
    pub v2: u8,
    pub v3: u8,
    pub v4: u8,
    pub v5: u8,
    pub v6: u8,
    pub v7: u8,
    pub v8: u8,
    pub v9: u8,
    pub va: u8,
    pub vb: u8,
    pub vc: u8,
    pub vd: u8,
    pub ve: u8,
    pub vf: u8, // carry flag

    pub i: u16, // can only be loaded with a 12-bit address value

    pub pc: u16,

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xE8F - Program ROM and work RAM
    // 0x390-0xFFF - 'variables and display refresh'
    pub memory: [u8; 4096],

    pub gfx: [u8; 64 * 32],

    pub delay_timer: u8,

    // The system’s buzzer sounds whenever the sound timer reaches zero.
    pub sound_timer: u8,

    pub stack: [u16; 16],
    pub sp: u16,

    pub _key: [u8; 16]
}

impl Default for Chip8 {
    fn default() -> Chip8 {
        Chip8 {
            v0: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
            v5: 0,
            v6: 0,
            v7: 0,
            v8: 0,
            v9: 0,
            va: 0,
            vb: 0,
            vc: 0,
            vd: 0,
            ve: 0,
            vf: 0,

            i:  0,

            pc: 0,

            delay_timer: 0,
            sound_timer: 0,
            memory: [0u8; 4096],
            stack: [0u16; 16],
            sp: 0,
            gfx: [0u8; (64 * 32)],
            _key: [0u8; 16],
        }
    }
}

impl Chip8 {
    pub fn initialize(&mut self) {
        self.pc = 0x200;
        self.i = 0;
        self.sp = 0;

        self.gfx = [0u8; (64 * 32)];
        self.stack = [0u16; 16];

        self.v0 = 0;
        self.v1 = 0;
        self.v2 = 0;
        self.v3 = 0;
        self.v4 = 0;
        self.v5 = 0;
        self.v6 = 0;
        self.v7 = 0;
        self.v8 = 0;
        self.v9 = 0;
        self.va = 0;
        self.vb = 0;
        self.vc = 0;
        self.vd = 0;
        self.ve = 0;
        self.vf = 0;

        // load fontset
        self.memory[0..80].copy_from_slice(&CHIP8_FONTSET[0..80]);
        // rest of memory is zeroed
        self.memory[80..4096].copy_from_slice(&[0u8;4016]);

        self.delay_timer = 0;
        self.sound_timer = 0;
    }

    // FIXME: error if the rom_data is too large for the memory space ( 0x200-0xE8F )
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory[0x200..(0x200 + rom_data.len())].copy_from_slice(&rom_data);
    }

    pub fn emulate_cycle(&mut self) {
        // fetch
        let opcode = ((self.memory[self.pc as usize] as u16) << 8) | (self.memory[(self.pc + 1) as usize] as u16);

        // decode & execute
        let mut advance = true;

        match opcode {
            0x00E0 => {
                // 00E0 - clear the screen
                self.gfx = [0u8; (64 * 32)];
            },
            0x1000...0x1FFF => {
                // 1NNN - goto
                self.pc = opcode & 0x0FFF;
                advance = false;
            },
            0x2000...0x2FFF => {
                // 2NNN - subroutine
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = opcode & 0x0FFF;
                advance = false;
            },
            0x6000...0x6FFF => {
                // 6XNN - store NN in register X

            },
            0xA000...0xAFFF => {
                // ANNN - store NNN in I
                self.i = opcode & 0x0FFF;
            },
            _ => panic!("unknown opcode {:x?}", opcode),
        };

        if advance {
            // increment program counter
            self.pc += 2;
        }


        // FIXME: should execute 60 cycles per second, delay until tick if not ready

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP");
            }
            self.sound_timer -= 1;
        }
    }

    // pub fn set_keys(&self) {

    // }
}
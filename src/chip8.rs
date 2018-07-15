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

pub const SCREEN_WIDTH: u32 = 64;
pub const SCREEN_HEIGHT: u32 = 32;

pub struct Chip8 {
    v: [u8; 15], // registers
    vf: u8, // carry flag

    i: u16, // can only be loaded with a 12-bit address value

    pc: u16, // program counter

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xE8F - Program ROM and work RAM
    // 0x390-0xFFF - 'variables and display refresh'
    memory: [u8; 4096],

    pub gfx: [u8; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],

    delay_timer: u8,

    // The system’s buzzer sounds whenever the sound timer reaches zero
    sound_timer: u8,

    stack: [u16; 16],
    sp: u16,

    pub _key: [u8; 16],

    draw: bool
}

impl Default for Chip8 {
    fn default() -> Chip8 {
        Chip8 {
            v: [0u8; 15],
            vf: 0,

            i:  0,

            pc: 0,

            delay_timer: 0,
            sound_timer: 0,
            memory: [0u8; 4096],
            stack: [0u16; 16],
            sp: 0,
            gfx: [0u8; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
            _key: [0u8; 16],

            draw: true,
        }
    }
}

impl Chip8 {
    pub fn initialize(&mut self) {
        self.pc = 0x200;
        self.i = 0;
        self.sp = 0;

        self.gfx = [0u8; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize];
        self.stack = [0u16; 16];

        self.v = [0u8; 15];
        self.vf = 0;

        // load fontset
        self.memory[0..80].copy_from_slice(&CHIP8_FONTSET[0..80]);
        // rest of memory is zeroed
        self.memory[80..4096].copy_from_slice(&[0u8;4016]);

        self.delay_timer = 0;
        self.sound_timer = 0;

        self.draw = true;
    }

    // FIXME: error if the rom_data is too large for the memory space ( 0x200-0xE8F )
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory[0x200..(0x200 + rom_data.len())].copy_from_slice(&rom_data);
    }

    pub fn emulate_cycle(&mut self) {
        // fetch
        let opcode = ((self.memory[self.pc as usize] as u16) << 8) | (self.memory[(self.pc + 1) as usize] as u16);

        // decode & execute

        match opcode {
            0x00E0 => {
                // 00E0 - clear the screen
                self.gfx = [0u8; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize];
                self.draw = true;

                self.pc += 2;
            },
            0x1000...0x1FFF => {
                // 1NNN - goto
                self.pc = opcode & 0x0FFF;
            },
            0x2000...0x2FFF => {
                // 2NNN - subroutine
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = opcode & 0x0FFF;
            },
            0x3000...0x3EFF => {
                // 3XNN - Skip the following instruction if the value of register VX equals NN
                let reg = (opcode & 0x0F00) >> 8;
                let val = (opcode & 0x00FF) as u8;

                if self.v[reg as usize] == val { self.pc += 4; } else { self.pc += 2; };
            },
            0x4000...0x4EFF => {
                // 4XNN - Skip the following instruction if the value of register VX is not equal to NN
                let reg = (opcode & 0x0F00) >> 8;
                let val = (opcode & 0x00FF) as u8;

                if self.v[reg as usize] != val { self.pc += 4; } else { self.pc += 2; };
            },
            0x6000...0x6EFF => {
                // 6XNN - store NN in register X
                let reg = (opcode & 0x0F00) >> 8;

                self.v[reg as usize] = (opcode & 0x00FF) as u8;
                self.pc += 2;
            },
            0x7000...0x7EFF => {
                // 7XNN - Add the value NN to register VX
                let reg = (opcode & 0x0F00) >> 8;

                let result_carry = self.v[reg as usize].overflowing_add((opcode & 0x00FF) as u8);
                self.v[reg as usize] = result_carry.0;
                self.vf = if result_carry.1 { 1 } else { 0 };
                self.pc += 2;
            },
            0xA000...0xAFFF => {
                // ANNN - store NNN in I
                self.i = opcode & 0x0FFF;
                self.pc += 2;
            },
            0xD000...0xDEEF => {
                // DXYN - Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
                // Set VF to 01 if any set pixels are changed to unset, and 00 otherwise

                let reg_x = (opcode & 0x0F00) >> 8;
                let reg_y = (opcode & 0x00F0) >> 4;
                let height = opcode & 0x000F;
                let mut pixel: u8;
                self.vf = 0;

                for yline in 0..height {
                    pixel = self.memory[(self.i + yline) as usize];
                    for xline in 0..8 {
                        let offset = (((self.v[reg_x as usize] as u16) + xline +
                                     (((self.v[reg_y as usize] as u16) + yline) * (SCREEN_WIDTH as u16)))) as usize;
                        if (pixel & (0x80 >> xline)) != 0 {
                            if self.gfx[offset] == 1 {
                                self.vf = 1;
                            }
                            self.gfx[offset] ^= 1;
                        }
                    }
                }

                self.draw = true;

                self.pc += 2;
            },
            _ => panic!("unknown opcode {:x?}", opcode),
        };

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

    pub fn graphics_needs_refresh(&self) -> bool {
        return self.draw;
    }

    pub fn graphics_clear_refresh(&mut self) {
        self.draw = false;
    }

    // pub fn set_keys(&self) {

    // }
}
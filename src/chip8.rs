extern crate rand;

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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub const SCREEN_WIDTH: u8 = 64;
pub const SCREEN_HEIGHT: u8 = 32;

pub struct Chip8 {
    v: [u8; 16], // registers

    i: u16, // can only be loaded with a 12-bit address value

    pc: u16, // program counter

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xE8F - Program ROM and work RAM
    // 0x390-0xFFF - 'variables and display refresh'
    memory: [u8; 4096],

    pub gfx: Vec<u8>,

    delay_timer: u8,

    // The system’s buzzer sounds whenever the sound timer reaches zero
    sound_timer: u8,

    stack: [u16; 16],
    sp: u16,

    key: [u8; 16],

    draw: bool,
}

impl Default for Chip8 {
    fn default() -> Chip8 {
        Chip8 {
            v: [0u8; 16],

            i: 0,

            pc: 0,

            delay_timer: 0,
            sound_timer: 0,
            memory: [0u8; 4096],
            stack: [0u16; 16],
            sp: 0,

            gfx: Vec::new(),

            key: [0u8; 16],

            draw: true,
        }
    }
}

impl Chip8 {
    pub fn initialize(&mut self) {
        self.pc = 0x200;
        self.i = 0;
        self.sp = 0;

        self.gfx = Vec::with_capacity(usize::from(SCREEN_WIDTH) * usize::from(SCREEN_HEIGHT));
        self.gfx.resize(self.gfx.capacity(), 0);

        self.stack = [0u16; 16];

        self.v = [0u8; 16];

        // load fontset
        self.memory[0..80].copy_from_slice(&CHIP8_FONTSET[0..80]);
        // rest of memory is zeroed
        self.memory[80..4096].copy_from_slice(&[0u8; 4016]);

        self.key = [0u8; 16];

        self.delay_timer = 0;
        self.sound_timer = 0;

        self.draw = true;
    }

    // FIXME: error if the rom_data is too large for the memory space ( 0x200-0xE8F )
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory[0x200..(0x200 + rom_data.len())].copy_from_slice(&rom_data);
    }

    pub fn emulate_cycle(&mut self, legacy_mode: bool) {
        let opcode = (u16::from(self.memory[self.pc as usize]) << 8)
            | u16::from(self.memory[(self.pc + 1) as usize]);

        match opcode {
            0x00E0 => {
                // 00E0 - clear the screen
                self.gfx = Vec::with_capacity(usize::from(SCREEN_WIDTH) * usize::from(SCREEN_HEIGHT));
                self.gfx.resize(self.gfx.capacity(), 0);
                self.draw = true;

                self.pc += 2;
            }
            0x00EE => {
                // 00EE - return from a subroutine
                // TODO: panic if stack is empty?
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize] + 2;
            }
            o if o & 0xF000 == 0x1000 => {
                // 1NNN - goto
                self.pc = o & 0x0FFF;
            }
            o if o & 0xF000 == 0x2000 => {
                // 2NNN - subroutine
                // TODO: panic if stack is full?
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = o & 0x0FFF;
            }
            o if o & 0xF000 == 0x3000 => {
                // 3XNN - Skip the following instruction if the value of register VX equals NN
                let reg = (o & 0x0F00) >> 8;
                let val = (o & 0x00FF) as u8;

                if self.v[reg as usize] == val {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                };
            }
            o if o & 0xF000 == 0x4000 => {
                // 4XNN - Skip the following instruction if the value of register VX is not equal to NN
                let reg = (o & 0x0F00) >> 8;
                let val = (o & 0x00FF) as u8;

                if self.v[reg as usize] != val {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                };
            }
            o if o & 0xF00F == 0x5000 => {
                // 5XY0 - Skip the following instruction if the value of register VX equals value of register VY
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                if self.v[reg_x as usize] == self.v[reg_y as usize] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                };
            }
            o if o & 0xF000 == 0x6000 => {
                // 6XNN - store NN in register X
                let reg = (o & 0x0F00) >> 8;

                self.v[reg as usize] = (o & 0x00FF) as u8;
                self.pc += 2;
            }
            o if o & 0xF000 == 0x7000 => {
                // 7XNN - Add the value NN to register VX (carry flag is not changed)
                let reg = (o & 0x0F00) >> 8;

                let result_carry = self.v[reg as usize].overflowing_add((o & 0x00FF) as u8);
                self.v[reg as usize] = result_carry.0;
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8000 => {
                // 8XY0 - Assign the value of register VX to the value of register VY
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                self.v[reg_x as usize] = self.v[reg_y as usize];
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8001 => {
                // 8XY1 - Bitwise OR the values of registers VX and register VY, result to VX
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                self.v[reg_x as usize] |= self.v[reg_y as usize];
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8002 => {
                // 8XY2 - Bitwise AND the values of registers VX and register VY, result to VX
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                self.v[reg_x as usize] &= self.v[reg_y as usize];
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8003 => {
                // 8XY3 - Bitwise XOR the values of registers VX and register VY, result to VX
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                self.v[reg_x as usize] ^= self.v[reg_y as usize];
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8004 => {
                // 8XY4 - Add the values of registers VX and register VY, result to VX
                // VF = 1 if overflow
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                let result_carry = self.v[reg_x as usize].overflowing_add(self.v[reg_y as usize]);
                self.v[reg_x as usize] = result_carry.0;
                self.v[15] = if result_carry.1 { 1 } else { 0 };
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8005 => {
                // 8XY5 - Subtract value of register VY from value of register VX, result to VX
                // VF = 1 if no borrow, 0 if there is
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                let result_borrow = self.v[reg_x as usize].overflowing_sub(self.v[reg_y as usize]);
                self.v[reg_x as usize] = result_borrow.0;
                self.v[15] = if result_borrow.1 { 0 } else { 1 };
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8006 => {
                // 8XY6 - Store the value of register VY shifted right one bit in register VX
                // Set register VF to the least significant bit prior to the shift
                // NB: modern interpreters seem to operate on reg_x only
                let reg_x = (o & 0x0F00) >> 8;
                let val = if legacy_mode {
                    let reg_y = (o & 0x00F0) >> 4;
                    self.v[reg_y as usize]
                } else {
                    self.v[reg_x as usize]
                };

                self.v[reg_x as usize] = val.checked_shr(1).unwrap_or(0);
                self.v[15] = val & 0x1;
                self.pc += 2;
            }
            o if o & 0xF00F == 0x8007 => {
                // 8XY7 - Subtract value of register VX from value of register VY, result to VX
                // VF = 1 if no borrow, 0 if there is
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                let result_borrow = self.v[reg_y as usize].overflowing_sub(self.v[reg_x as usize]);
                self.v[reg_x as usize] = result_borrow.0;
                self.v[15] = if result_borrow.1 { 0 } else { 1 };
                self.pc += 2;
            }
            o if o & 0xF00F == 0x800E => {
                let reg_x = (o & 0x0F00) >> 8;

                let val = if legacy_mode {
                    let reg_y = (o & 0x00F0) >> 4;
                    self.v[reg_y as usize]
                } else {
                    self.v[reg_x as usize]
                };

                self.v[reg_x as usize] = val.checked_shl(1).unwrap_or(u8::max_value());
                self.v[15] = val & 0x80;
                self.pc += 2;
            }
            o if o & 0xF00F == 0x9000 => {
                // 9XY0 - Skip the following instruction if the value of register VX does not equal value of register VY
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;

                if self.v[reg_x as usize] == self.v[reg_y as usize] {
                    self.pc += 2;
                } else {
                    self.pc += 4;
                };
            }
            o if o & 0xF000 == 0xA000 => {
                // ANNN - store NNN in I
                self.i = o & 0x0FFF;
                self.pc += 2;
            }
            o if o & 0xF000 == 0xB000 => {
                // BNNN - goto NNN + V0
                self.pc = o + u16::from(self.v[0]);
            }
            o if o & 0xF000 == 0xC000 => {
                // CXNN - Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN.
                let reg = (o & 0x0F00) >> 8;
                let val = (o & 0x00FF) as u8;

                self.v[reg as usize] = val & rand::random::<u8>();
                self.pc += 2;
            }
            o if o & 0xF000 == 0xD000 => {
                // DXYN - Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
                // Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
                let reg_x = (o & 0x0F00) >> 8;
                let reg_y = (o & 0x00F0) >> 4;
                let height = o & 0x000F;
                let mut pixel: u8;
                self.v[15] = 0;

                for yline in 0..height {
                    pixel = self.memory[(self.i + yline) as usize];
                    for xline in 0..8 {
                        let offset = (u16::from(self.v[reg_x as usize])
                            + xline
                            + (u16::from(self.v[reg_y as usize]) + yline) * u16::from(SCREEN_WIDTH))
                            as usize;
                        if (pixel & (0x80 >> xline)) != 0 {
                            if self.gfx[offset] == 1 {
                                self.v[15] = 1;
                            }
                            self.gfx[offset] ^= 1;
                        }
                    }
                }

                self.draw = true;

                self.pc += 2;
            }
            o if o & 0xF0FF == 0xE09E => {
                // EX9E - Skip the following instruction if the key corresponding to the
                // value currently stored in register VX is pressed
                let reg = (o & 0x0F00) >> 8;

                if self.key[self.v[reg as usize] as usize] == 1 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                };
            }
            o if o & 0xF0FF == 0xE0A1 => {
                // EXA1 - Skip the following instruction if the key corresponding to the
                // value currently stored in register VX is not pressed
                let reg = (o & 0x0F00) >> 8;

                if self.key[self.v[reg as usize] as usize] == 0 {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                };
            }
            o if o & 0xF0FF == 0xF007 => {
                // FX07 - Store the current value of the delay timer in register VX
                let reg = (o & 0x0F00) >> 8;

                self.v[reg as usize] = self.delay_timer;
                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF00A => {
                // FX0A - Wait for a keypress and store the result in register VX
                let reg = (o & 0x0F00) >> 8;

                if let Some(num) = self.key.iter().position(|&k| k == 1) {
                    self.v[reg as usize] = num as u8;

                    self.pc += 2;
                }
            }
            o if o & 0xF0FF == 0xF015 => {
                // FX15 - Set the delay timer to the value of register VX
                let reg = (o & 0x0F00) >> 8;

                self.delay_timer = self.v[reg as usize];
                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF018 => {
                // FX18 - Set the sound timer to the value of register VX
                let reg = (o & 0x0F00) >> 8;

                self.sound_timer = self.v[reg as usize];
                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF01E => {
                // FX1E - Add the value stored in register VX to register I
                // Sets carry flag if 12-bit limit exceeded for I
                let reg = (o & 0x0F00) >> 8;

                self.i += u16::from(self.v[reg as usize]);

                if self.i > 0xFFF {
                    self.i -= 0x1000;
                    self.v[15] = 1;
                }

                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF029 => {
                // FX29 - Set I to the memory address of the sprite data corresponding to the
                // hexadecimal digit stored in register VX
                let reg = (o & 0x0F00) >> 8;

                // TODO: error if register value > 0x0F
                self.i = u16::from(5 * self.v[reg as usize]);

                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF033 => {
                // FX33 - Store the binary-coded decimal equivalent of the value stored in
                // register VX at addresses I, I + 1, and I + 2
                let reg = (o & 0x0F00) >> 8;

                let ones = self.v[reg as usize] % 10;
                let tens = ((self.v[reg as usize] % 100) - ones) / 10;
                let hundreds = (self.v[reg as usize] - (tens + ones)) / 100;

                self.memory[self.i as usize] = hundreds;
                self.memory[(self.i + 1) as usize] = tens;
                self.memory[(self.i + 2) as usize] = ones;

                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF055 => {
                // FX55 - Store the values of registers V0 to VX inclusive in memory starting at address I
                // I is set to I + X + 1 after operation
                let reg_num = (o & 0x0F00) >> 8;
                self.memory[(self.i as usize)..=(self.i + reg_num) as usize]
                    .copy_from_slice(&self.v[0..=(reg_num as usize)]);
                self.i += reg_num + 1;

                self.pc += 2;
            }
            o if o & 0xF0FF == 0xF065 => {
                // FX65 - Fill registers V0 to VX inclusive with the values stored in memory starting at address I
                // I is set to I + X + 1 after operation
                let reg_num = (o & 0x0F00) >> 8;
                self.v[0..=(reg_num as usize)]
                    .copy_from_slice(&self.memory[(self.i as usize)..=(self.i + reg_num) as usize]);
                self.i += reg_num + 1;

                self.pc += 2;
            }
            o => panic!("unknown opcode {:x?}", o),
        };

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn graphics_needs_refresh(&self) -> bool {
        self.draw
    }

    pub fn graphics_clear_refresh(&mut self) {
        self.draw = false;
    }

    pub fn key_down(&mut self, key_num: u8) {
        self.key[key_num as usize] = 1;
    }

    pub fn key_up(&mut self, key_num: u8) {
        self.key[key_num as usize] = 0;
    }

    pub fn audio_sound(&self) -> bool {
        self.sound_timer > 0
    }
}

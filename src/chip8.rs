use anyhow::{Context, Result};
use lazy_static::lazy_static;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::collections::HashMap;

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
const GRAPHICS_SIZE: usize = SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize;

lazy_static! {
    // maps SDL keycodes to Chip8 key input values
    static ref KEY_MAPPING: HashMap<Keycode, u8> = {
        let mut m = HashMap::new();
        m.insert(Keycode::Num1, 0x1);
        m.insert(Keycode::Num2, 0x2);
        m.insert(Keycode::Num3, 0x3);
        m.insert(Keycode::Num4, 0xc);
        m.insert(Keycode::Q, 0x4);
        m.insert(Keycode::W, 0x5);
        m.insert(Keycode::E, 0x6);
        m.insert(Keycode::R, 0xd);
        m.insert(Keycode::A, 0x7);
        m.insert(Keycode::S, 0x8);
        m.insert(Keycode::D, 0x9);
        m.insert(Keycode::F, 0xe);
        m.insert(Keycode::Z, 0xa);
        m.insert(Keycode::X, 0x0);
        m.insert(Keycode::C, 0xb);
        m.insert(Keycode::V, 0xf);
        m
    };
}

fn register_x(o: u16) -> usize {
    usize::from((o & 0x0F00) >> 8)
}

fn register_y(o: u16) -> usize {
    usize::from((o & 0x00F0) >> 4)
}

fn register_xy(o: u16) -> (usize, usize) {
    (
        usize::from((o & 0x0F00) >> 8),
        usize::from((o & 0x00F0) >> 4),
    )
}

fn opcode_value(o: u16) -> u8 {
    (o & 0x00FF) as u8
}

pub struct Chip8 {
    v: [u8; 16], // registers

    i: u16, // can only be loaded with a 12-bit address value

    pc: u16, // program counter

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xE8F - Program ROM and work RAM
    // 0x390-0xFFF - 'variables and display refresh'
    memory: [u8; 4096],

    pub gfx: [u8; GRAPHICS_SIZE],

    delay_timer: u8,

    // The system’s buzzer sounds whenever the sound timer reaches zero
    sound_timer: u8,

    stack: [u16; 16],
    sp: u8,

    key: [u8; 16],

    draw: bool,

    legacy_mode: bool,
}

impl Chip8 {
    pub fn new(legacy_mode: bool) -> Self {
        let mut memory = [0_u8; 4096];
        memory[0..80].copy_from_slice(&CHIP8_FONTSET[0..80]); // load fontset
        Self {
            legacy_mode,
            memory,

            v: [0_u8; 16],

            i: 0,

            pc: 0x200,

            delay_timer: 0,
            sound_timer: 0,
            stack: [0_u16; 16],
            sp: 0,
            gfx: [0_u8; GRAPHICS_SIZE],
            key: [0_u8; 16],

            draw: true,
        }
    }

    // FIXME: error if the rom_data is too large for the memory space ( 0x200-0xE8F )
    pub fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory[0x200..(0x200 + rom_data.len())].copy_from_slice(rom_data);
    }

    pub fn graphics_needs_refresh(&self) -> bool {
        self.draw
    }

    pub fn graphics_clear_refresh(&mut self) {
        self.draw = false;
    }

    pub fn audio_sound(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn handle_key(&mut self, event: &Event) -> Result<bool> {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                return Ok(true);
            }
            Event::KeyDown {
                repeat: false,
                keycode: Some(code),
                ..
            }
            | Event::KeyUp {
                repeat: false,
                keycode: Some(code),
                ..
            } if KEY_MAPPING.contains_key(code) => {
                let key_num = KEY_MAPPING.get(code).context("invalid key code")?;
                match event {
                    Event::KeyDown { .. } => self.key[usize::from(*key_num)] = 1,
                    Event::KeyUp { .. } => self.key[usize::from(*key_num)] = 0,
                    _ => unreachable!(),
                };
            }
            _ => {}
        };

        Ok(false)
    }

    pub fn emulate_cycle(&mut self) {
        let opcode = (u16::from(self.memory[self.pc as usize]) << 8)
            | u16::from(self.memory[(self.pc + 1) as usize]);

        if match opcode {
            0x00EE => self.c8_flow_return(),
            o if o & 0xF000 == 0x1000 => self.c8_flow_goto(o),
            o if o & 0xF000 == 0x2000 => self.c8_flow_gosub(o),
            o if o & 0xF000 == 0xB000 => self.c8_flow_jump(o),
            _ => false,
        } {
            return;
        }

        self.pc += match opcode {
            0x00E0 => self.c8_display_clear(),

            // 0NNN is ignored by modern interpreters
            o if o & 0xF000 == 0x0000 => 0,

            o if o & 0xF000 == 0x3000 => self.c8_cond_skip_eq_num(o),
            o if o & 0xF000 == 0x4000 => self.c8_cond_skip_neq_num(o),

            o if o & 0xF00F == 0x5000 => self.c8_cond_skip_eq_reg(o),

            o if o & 0xF000 == 0x6000 => self.c8_const_set_num(o),
            o if o & 0xF000 == 0x7000 => self.c8_const_add_num(o),

            o if o & 0xF00F == 0x8000 => self.c8_assign_set_reg(o),
            o if o & 0xF00F == 0x8001 => self.c8_bitop_or_reg(o),
            o if o & 0xF00F == 0x8002 => self.c8_bitop_and_reg(o),
            o if o & 0xF00F == 0x8003 => self.c8_bitop_xor_reg(o),
            o if o & 0xF00F == 0x8004 => self.c8_math_add_reg(o),
            o if o & 0xF00F == 0x8005 => self.c8_math_sub_reg(o),
            o if o & 0xF00F == 0x8006 => self.c8_bitop_shr_reg(o),
            o if o & 0xF00F == 0x8007 => self.c8_math_neg_reg(o),
            o if o & 0xF00F == 0x800E => self.c8_bitop_shl_reg(o),

            o if o & 0xF00F == 0x9000 => self.c8_cond_skipifneq_reg(o),
            o if o & 0xF000 == 0xA000 => self.c8_mem_store(o),

            o if o & 0xF000 == 0xC000 => self.c8_rand_and_reg(o),

            o if o & 0xF000 == 0xD000 => self.c8_draw_sprite(o),

            o if o & 0xF0FF == 0xE09E => self.c8_key_pressedskip(o),
            o if o & 0xF0FF == 0xE0A1 => self.c8_key_notpressedskip(o),

            o if o & 0xF0FF == 0xF00A => self.c8_key_wait(o),

            o if o & 0xF0FF == 0xF007 => self.c8_timer_delay_store(o),
            o if o & 0xF0FF == 0xF015 => self.c8_timer_delay_set(o),
            o if o & 0xF0FF == 0xF018 => self.c8_timer_sound_set(o),

            o if o & 0xF0FF == 0xF01E => self.c8_mem_addi(o),

            o if o & 0xF0FF == 0xF029 => self.c8_mem_spriteaddr(o),
            o if o & 0xF0FF == 0xF033 => self.c8_bcd_store(o),

            o if o & 0xF0FF == 0xF055 => self.c8_mem_reg_dump(o),
            o if o & 0xF0FF == 0xF065 => self.c8_mem_reg_load(o),
            o => panic!("unknown opcode {:x?}", o),
        };
    }

    fn c8_flow_return(&mut self) -> bool {
        // 00EE - return from a subroutine
        // TODO: panic if stack is empty?
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize] + 2;
        true
    }

    fn c8_flow_goto(&mut self, o: u16) -> bool {
        // 1NNN - goto
        self.pc = o & 0x0FFF;
        true
    }

    fn c8_flow_gosub(&mut self, o: u16) -> bool {
        // 2NNN - subroutine
        // TODO: panic if stack is full?
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = o & 0x0FFF;
        true
    }

    fn c8_display_clear(&mut self) -> u16 {
        // 00E0 - clear the screen
        self.gfx = [0_u8; GRAPHICS_SIZE];
        self.draw = true;

        2
    }

    fn c8_cond_skip_eq_num(&mut self, o: u16) -> u16 {
        // 3XNN - Skip the following instruction if the value of register VX equals NN
        let reg = register_x(o);
        let val = opcode_value(o);

        if self.v[reg] == val {
            4
        } else {
            2
        }
    }

    fn c8_cond_skip_neq_num(&mut self, o: u16) -> u16 {
        // 4XNN - Skip the following instruction if the value of register VX is not equal to NN
        let reg = register_x(o);
        let val = opcode_value(o);

        if self.v[reg] == val {
            2
        } else {
            4
        }
    }

    fn c8_cond_skip_eq_reg(&mut self, o: u16) -> u16 {
        // 5XY0 - Skip the following instruction if the value of register VX equals value of register VY
        let (reg_x, reg_y) = register_xy(o);

        if self.v[reg_x] == self.v[reg_y] {
            4
        } else {
            2
        }
    }

    fn c8_const_set_num(&mut self, o: u16) -> u16 {
        // 6XNN - store NN in register X
        let reg = register_x(o);

        self.v[reg] = (o & 0x00FF) as u8;
        2
    }

    fn c8_const_add_num(&mut self, o: u16) -> u16 {
        // 7XNN - Add the value NN to register VX (carry flag is not changed)
        let reg = register_x(o);

        let result_carry = self.v[reg].overflowing_add((o & 0x00FF) as u8);
        self.v[reg] = result_carry.0;
        2
    }

    fn c8_assign_set_reg(&mut self, o: u16) -> u16 {
        // 8XY0 - Assign the value of register VX to the value of register VY
        let (reg_x, reg_y) = register_xy(o);

        self.v[reg_x] = self.v[reg_y];
        2
    }

    fn c8_bitop_or_reg(&mut self, o: u16) -> u16 {
        // 8XY1 - Bitwise OR the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = register_xy(o);

        self.v[reg_x] |= self.v[reg_y];
        2
    }

    fn c8_bitop_and_reg(&mut self, o: u16) -> u16 {
        // 8XY2 - Bitwise AND the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = register_xy(o);

        self.v[reg_x] &= self.v[reg_y];
        2
    }

    fn c8_bitop_xor_reg(&mut self, o: u16) -> u16 {
        // 8XY3 - Bitwise XOR the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = register_xy(o);

        self.v[reg_x] ^= self.v[reg_y];
        2
    }

    fn c8_math_add_reg(&mut self, o: u16) -> u16 {
        // 8XY4 - Add the values of registers VX and register VY, result to VX
        // VF = 1 if overflow
        let (reg_x, reg_y) = register_xy(o);

        let result_carry = self.v[reg_x].overflowing_add(self.v[reg_y]);
        self.v[reg_x] = result_carry.0;
        self.v[15] = if result_carry.1 { 1 } else { 0 };
        2
    }

    fn c8_math_sub_reg(&mut self, o: u16) -> u16 {
        // 8XY5 - Subtract value of register VY from value of register VX, result to VX
        // VF = 1 if no borrow, 0 if there is
        let (reg_x, reg_y) = register_xy(o);

        let result_borrow = self.v[reg_x].overflowing_sub(self.v[reg_y]);
        self.v[reg_x] = result_borrow.0;
        self.v[15] = if result_borrow.1 { 0 } else { 1 };
        2
    }

    fn c8_bitop_shr_reg(&mut self, o: u16) -> u16 {
        // 8XY6 - Store the value of register VY shifted right one bit in register VX
        // Set register VF to the least significant bit prior to the shift
        // NB: modern interpreters seem to operate on reg_x only
        let reg_x = register_x(o);

        let val = if self.legacy_mode {
            let reg_y = register_y(o);
            self.v[reg_y]
        } else {
            self.v[reg_x]
        };

        self.v[reg_x] = val.checked_shr(1).unwrap_or(0);
        self.v[15] = val & 0x1;
        2
    }

    fn c8_math_neg_reg(&mut self, o: u16) -> u16 {
        // 8XY7 - Subtract value of register VX from value of register VY, result to VX
        // VF = 1 if no borrow, 0 if there is
        let (reg_x, reg_y) = register_xy(o);

        let result_borrow = self.v[reg_y].overflowing_sub(self.v[reg_x]);
        self.v[reg_x] = result_borrow.0;
        self.v[15] = if result_borrow.1 { 0 } else { 1 };
        2
    }

    fn c8_bitop_shl_reg(&mut self, o: u16) -> u16 {
        let reg_x = register_x(o);

        let val = if self.legacy_mode {
            let reg_y = register_y(o);
            self.v[reg_y]
        } else {
            self.v[reg_x]
        };

        self.v[reg_x] = val.checked_shl(1).unwrap_or(u8::max_value());
        self.v[15] = val & 0x80;
        2
    }

    fn c8_cond_skipifneq_reg(&mut self, o: u16) -> u16 {
        // 9XY0 - Skip the following instruction if the value of register VX does not equal value of register VY
        let (reg_x, reg_y) = register_xy(o);

        if self.v[reg_x] == self.v[reg_y] {
            2
        } else {
            4
        }
    }

    pub fn c8_mem_store(&mut self, o: u16) -> u16 {
        // ANNN - store NNN in I
        self.i = o & 0x0FFF;
        2
    }

    fn c8_flow_jump(&mut self, o: u16) -> bool {
        // BNNN - goto NNN + V0
        self.pc = o + u16::from(self.v[0]);
        true
    }

    pub fn c8_rand_and_reg(&mut self, o: u16) -> u16 {
        // CXNN - Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN.
        let reg = register_x(o);
        let val = opcode_value(o);

        self.v[reg] = val & rand::random::<u8>();
        2
    }

    fn c8_draw_sprite(&mut self, o: u16) -> u16 {
        // DXYN - Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
        // Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
        let (reg_x, reg_y) = register_xy(o);
        let height = o & 0x000F;

        let mut pixel: u8;
        let mut offset: usize;
        self.v[15] = 0;

        for yline in 0..height {
            pixel = self.memory[(self.i + yline) as usize];
            for xline in 0..8 {
                if (pixel & (0x80 >> xline)) != 0 {
                    offset = (u16::from(self.v[reg_x])
                        + xline
                        + ((u16::from(self.v[reg_y]) + yline) * u16::from(SCREEN_WIDTH)))
                        as usize;
                    if self.gfx[offset] == 1 {
                        self.v[15] = 1;
                    }
                    self.gfx[offset] ^= 1;
                }
            }
        }

        self.draw = true;

        2
    }

    fn c8_key_pressedskip(&mut self, o: u16) -> u16 {
        // EX9E - Skip the following instruction if the key corresponding to the
        // value currently stored in register VX is pressed
        let reg = register_x(o);

        if self.key[self.v[reg] as usize] == 1 {
            4
        } else {
            2
        }
    }

    fn c8_key_notpressedskip(&mut self, o: u16) -> u16 {
        // EXA1 - Skip the following instruction if the key corresponding to the
        // value currently stored in register VX is not pressed
        let reg = register_x(o);

        if self.key[self.v[reg] as usize] == 0 {
            4
        } else {
            2
        }
    }

    fn c8_key_wait(&mut self, o: u16) -> u16 {
        // FX0A - Wait for a keypress and store the result in register VX
        let reg = register_x(o);

        // key position is in range 0..15, so this shouldn't cause problems
        match self.key.iter().position(|&k| k == 1) {
            Some(num) if num < self.key.len() => {
                self.v[reg] = u8::try_from(num).unwrap();

                2
            }
            _ => 0,
        }
    }

    fn c8_timer_delay_store(&mut self, o: u16) -> u16 {
        // FX07 - Store the current value of the delay timer in register VX
        let reg_x = register_x(o);

        self.v[reg_x] = self.delay_timer;
        2
    }

    fn c8_timer_delay_set(&mut self, o: u16) -> u16 {
        // FX15 - Set the delay timer to the value of register VX
        let reg_x = register_x(o);

        self.delay_timer = self.v[reg_x];
        2
    }

    fn c8_timer_sound_set(&mut self, o: u16) -> u16 {
        // FX18 - Set the sound timer to the value of register VX
        let reg_x = register_x(o);

        self.sound_timer = self.v[reg_x];
        2
    }

    fn c8_mem_addi(&mut self, o: u16) -> u16 {
        // FX1E - Add the value stored in register VX to register I
        // Sets carry flag if 12-bit limit exceeded for I
        let reg_x = register_x(o);

        self.i += u16::from(self.v[reg_x]);

        if self.i > 0xFFF {
            self.i -= 0x1000;
            self.v[15] = 1;
        }

        2
    }

    fn c8_mem_spriteaddr(&mut self, o: u16) -> u16 {
        // FX29 - Set I to the memory address of the sprite data corresponding to the
        // hexadecimal digit stored in register VX
        let reg_x = register_x(o);

        // TODO: error if register value > 0x0F
        self.i = u16::from(5 * self.v[reg_x]);
        2
    }

    fn c8_bcd_store(&mut self, o: u16) -> u16 {
        // FX33 - Store the binary-coded decimal equivalent of the value stored in
        // register VX at addresses I, I + 1, and I + 2
        let reg_x = register_x(o);

        let ones = self.v[reg_x] % 10;
        let tens = ((self.v[reg_x] % 100) - ones) / 10;
        let hundreds = (self.v[reg_x] - (tens + ones)) / 100;

        self.memory[self.i as usize] = hundreds;
        self.memory[(self.i + 1) as usize] = tens;
        self.memory[(self.i + 2) as usize] = ones;
        2
    }

    fn c8_mem_reg_dump(&mut self, o: u16) -> u16 {
        // FX55 - Store the values of registers V0 to VX inclusive in memory starting at address I
        // I is set to I + X + 1 after operation
        let reg_num = (o & 0x0F00) >> 8;
        self.memory[(self.i as usize)..=(self.i + reg_num) as usize]
            .copy_from_slice(&self.v[0..=(reg_num as usize)]);
        self.i += reg_num + 1;
        2
    }

    fn c8_mem_reg_load(&mut self, o: u16) -> u16 {
        // FX65 - Fill registers V0 to VX inclusive with the values stored in memory starting at address I
        // I is set to I + X + 1 after operation
        let reg_num = (o & 0x0F00) >> 8;
        self.v[0..=(reg_num as usize)]
            .copy_from_slice(&self.memory[(self.i as usize)..=(self.i + reg_num) as usize]);
        self.i += reg_num + 1;
        2
    }
}

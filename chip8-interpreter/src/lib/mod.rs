use std::sync::Arc;
use std::{sync::atomic::AtomicU8, thread, time::Duration};

pub struct CHIP8 {
    memory: [u8; 0x1000],
    pc: u16,
    stack: [u16; 24],
    sp: usize,
    op: u16,
    key: Option<u8>,
    registers: [u8; 16],
    i: u16,
    rng: RNG,
    delay_timer: Arc<AtomicU8>, // 60hz
    key_wait_target: Option<usize>,
    _thread: Option<std::thread::JoinHandle<()>>,
    pub draw_flag: bool,
    pub clear_flag: bool,
}

impl CHIP8 {
    const DISPLAY_START: usize = 0x1000 - 0x100;

    fn _00e0(&mut self) {
        // clear the screen.
        for i in Self::DISPLAY_START..self.memory.len() {
            self.memory[i] = 0;
        }
        self.clear_flag = true;
    }

    fn _00ee(&mut self) {
        // return
        self.sp -= 1;
        self.pc = self.stack[self.sp];
    }

    fn _1nnn(&mut self) {
        // jump nnn
        self.pc = self.op & 0xfff;
    }

    fn _2nnn(&mut self) {
        // call nnn
        self.stack[self.sp] = self.pc;
        self.sp += 1;
        self.pc = self.op & 0xfff;
    }

    fn _3xnn(&mut self) {
        // skip if vx == nn
        let x = (self.op >> 8) & 0xf;
        let x = self.registers[x as usize];
        let nn = (self.op & 0xff) as u8;
        if x == nn {
            self.pc = self.pc.wrapping_add(2);
        }
    }

    fn _4xnn(&mut self) {
        // skip if vx != nn
        let x = (self.op >> 8) & 0xf;
        let x = self.registers[x as usize];
        let nn = (self.op & 0xff) as u8;
        if x != nn {
            self.pc = self.pc.wrapping_add(2);
        }
    }

    fn _5xy0(&mut self) {
        // skip if vx == vy
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let (x, y) = (self.registers[x as usize], self.registers[y as usize]);
        if x == y {
            self.pc = self.pc.wrapping_add(2);
        }
    }

    fn _6xnn(&mut self) {
        // vx = nn.
        let x = (self.op >> 8) & 0xf;
        let nn = (self.op & 0xff) as u8;
        self.registers[x as usize] = nn;
    }

    fn _7xnn(&mut self) {
        // vx += nn (no carry set).
        let x = (self.op >> 8) & 0xf;
        let nn = (self.op & 0xff) as u8;
        self.registers[x as usize] = self.registers[x as usize].wrapping_add(nn);
    }

    fn _8xy0(&mut self) {
        // vx = vy.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        self.registers[x as usize] = self.registers[y as usize];
    }

    fn _8xy1(&mut self) {
        // vx |= vy.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        self.registers[x as usize] |= self.registers[y as usize];
    }

    fn _8xy2(&mut self) {
        // vx &= vy.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        self.registers[x as usize] &= self.registers[y as usize];
    }

    fn _8xy3(&mut self) {
        // vx ^= vy.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        self.registers[x as usize] ^= self.registers[y as usize];
    }

    fn _8xy4(&mut self) {
        // vx += vy (sets VF if overflow occurs).
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let (xv, overflowed) =
            self.registers[x as usize].overflowing_add(self.registers[y as usize]);
        self.registers[x as usize] = xv;
        self.registers[0xf] = if overflowed { 1 } else { 0 };
    }

    fn _8xy5(&mut self) {
        // vx -= vy (sets VF if no overflow occurs).
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let y = self.registers[y as usize];
        let (xv, overflowed) = self.registers[x as usize].overflowing_add(!y + 1);
        self.registers[x as usize] = xv;
        self.registers[0xf] = if !overflowed { 1 } else { 0 };
    }

    fn _8xy6(&mut self) {
        // shift right vx into vy and store the overflowed bit into VF.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let (yv, overflowed) = self.registers[x as usize].overflowing_shr(1);
        self.registers[y as usize] = yv;
        self.registers[0xf] = if overflowed { 1 } else { 0 };
    }

    fn _8xy7(&mut self) {
        // reversed add: x = y - x;
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let xv = self.registers[x as usize];
        let (xv, overflowed) = self.registers[y as usize].overflowing_add(!xv + 1);
        self.registers[x as usize] = xv;
        self.registers[0xf] = if !overflowed { 1 } else { 0 };
    }

    fn _8xye(&mut self) {
        // shift left vx into vy and store the overflowed bit into VF.
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        let (yv, overflowed) = self.registers[x as usize].overflowing_shl(1);
        self.registers[y as usize] = yv;
        self.registers[0xf] = if overflowed { 1 } else { 0 };
    }

    fn _9xy0(&mut self) {
        // skip if vx != vy
        let (x, y) = (self.op >> 8 & 0xf, self.op >> 4 & 0xf);
        if self.registers[x as usize] != self.registers[y as usize] {
            self.pc = self.pc.wrapping_add(2);
        }
    }

    fn _annn(&mut self) {
        // set I to nnn.
        self.i = self.op & 0xfff;
    }

    fn _bnnn(&mut self) {
        // jump to nnn + v0.
        let v0 = self.registers[0] as u16;
        self.pc = (self.op & 0xfff).wrapping_add(v0);
    }

    fn _cxnn(&mut self) {
        // random & nn -> vx
        let x = self.op >> 8 & 0xf;
        let nn = (self.op & 0xff) as u8;
        let next = self.rng.0 & nn;
        self.registers[x as usize] = next;
    }

    pub fn line_at(&self, addr: isize) -> u64 {
        debug_assert!(addr <= 31, "Out of bounds");
        // give the u64 of the line.
        let ptr = self.screen().as_ptr() as *const u64;
        unsafe { (*ptr.offset(addr)).reverse_bits() }
    }

    fn write_line_at(&mut self, addr: isize, line: u64) {
        debug_assert!(addr <= 31, "Out of bounds");
        let ptr = self.memory[Self::DISPLAY_START..].as_mut_ptr() as *mut u64;
        unsafe {
            let ptr = ptr.offset(addr);
            *ptr = line.reverse_bits();
        }
    }

    fn _dxyn(&mut self) {
        // draw at x, y, with n height.
        let x = self.op >> 8 & 0xf;
        let y = self.op >> 4 & 0xf;
        let n = self.op & 0xf;

        let x = self.registers[x as usize] as u32 % 64;
        let y = self.registers[y as usize] as isize % 32;

        let lshift = 63 - x - 8; // I want to align the rightmost bit, not the left most, that's why
                                 // - 7.
        let mut flag = 0;
        for i in 0..n {
            let sprite_i = self.i + i;
            let line = self.line_at(y + i as isize);
            let target = (self.memory[sprite_i as usize] as u64) << lshift;
            let result = line ^ target;
            if result != line | target {
                flag = 1;
            }
            self.write_line_at(y + i as isize, result);
        }
        self.draw_flag = true;
        self.registers[0xf] = flag;
    }
    fn _ex9e(&mut self) {
        // if key == vx then skip
        let x = self.op >> 8 & 0xf;
        if matches!(
            self.key.filter(|k| self.registers[x as usize] == *k),
            Some(_)
        ) {
            self.pc += 2;
        }
    }
    fn _exa1(&mut self) {
        // if key != vx then skip
        let x = self.op >> 8 & 0xf;
        if matches!(
            self.key.filter(|k| self.registers[x as usize] != *k),
            Some(_)
        ) {
            self.pc += 2;
        }
    }
    fn _fx07(&mut self) {
        let x = self.op >> 8 & 0xf;
        if self.registers[x as usize] == self.delay_timer.load(std::sync::atomic::Ordering::SeqCst)
        {
            self.pc += 2;
        }
    }

    fn _fx0a(&mut self) {
        // blocks until a key is pressed.
        self.key_wait_target = Some((self.op >> 8 & 0xf) as usize);
    }

    fn _fx15(&mut self) {
        // sets delay timer to vx.
        let x = self.op >> 8 & 0xf;
        self.delay_timer.store(
            self.registers[x as usize],
            std::sync::atomic::Ordering::SeqCst,
        );
    }

    // fx18 not implemented as not dealing with sounds :|

    fn _fx1e(&mut self) {
        // I += vx;
        let x = self.op >> 8 & 0xf;
        self.i = self.i.wrapping_add(x);
    }

    fn _fx29(&mut self) {
        // i = font[vx]
        let x = self.registers[(self.op >> 8 & 0xf) as usize] as u16 & 0xf;
        self.i = x * 5;
    }

    fn _fx33(&mut self) {
        // bcd
        let x = self.op >> 8 & 0xf;
        let mut v = self.registers[x as usize];
        for i in (0..3).rev() {
            self.memory[self.i as usize + i] = v % 10;
            v /= 10;
        }
    }

    fn _fx55(&mut self) {
        // dump registers until (and including) vx.
        let x = (self.op >> 8 & 0xf) as usize;
        for i in 0..=x {
            self.memory[self.i as usize + i] = self.registers[i];
        }
    }

    fn _fx65(&mut self) {
        // same as above, but loading
        let x = (self.op >> 8 & 0xf) as usize;
        for i in 0..=x as usize {
            self.registers[i] = self.memory[self.i as usize + i];
        }
    }

    fn exec(&mut self) {
        let (a, c, d) = (self.op >> 12, self.op >> 4 & 0xf, self.op & 0xf);
        match (a, c, d) {
            (0, 0xe, 0) => self._00e0(),
            (0, 0xe, 0xe) => self._00ee(),
            (1, _, _) => self._1nnn(),
            (2, _, _) => self._2nnn(),
            (3, _, _) => self._3xnn(),
            (4, _, _) => self._4xnn(),
            (5, _, 0) => self._5xy0(),
            (6, _, _) => self._6xnn(),
            (7, _, _) => self._7xnn(),
            (8, _, 0) => self._8xy0(),
            (8, _, 1) => self._8xy1(),
            (8, _, 2) => self._8xy2(),
            (8, _, 3) => self._8xy3(),
            (8, _, 4) => self._8xy4(),
            (8, _, 5) => self._8xy5(),
            (8, _, 6) => self._8xy6(),
            (8, _, 7) => self._8xy7(),
            (8, _, 0xe) => self._8xye(),
            (9, _, 0) => self._9xy0(),
            (0xa, _, _) => self._annn(),
            (0xb, _, _) => self._bnnn(),
            (0xc, _, _) => self._cxnn(),
            (0xd, _, _) => self._dxyn(),
            (0xe, 9, 0xe) => self._ex9e(),
            (0xe, 0xa, 1) => self._exa1(),
            (0xf, 0, 7) => self._fx07(),
            (0xf, 0, 0xa) => self._fx0a(),
            (0xf, 1, 5) => self._fx15(),
            (0xf, 1, 0xe) => self._fx1e(),
            (0xf, 2, 9) => self._fx29(),
            (0xf, 3, 3) => self._fx33(),
            (0xf, 5, 5) => self._fx55(),
            (0xf, 6, 5) => self._fx65(),
            _ => {}
        }
    }

    fn fetch(&mut self) {
        self.op =
            (self.memory[self.pc as usize] as u16) << 8 | self.memory[self.pc as usize + 1] as u16;
        self.pc = self.pc.wrapping_add(2);
    }

    pub fn key(&mut self, k: u8) {
        self.key = match k {
            b'1' => Some(1),
            b'2' => Some(2),
            b'3' => Some(3),
            b'q' => Some(4),
            b'w' => Some(5),
            b'e' => Some(6),
            b'a' => Some(7),
            b's' => Some(8),
            b'd' => Some(9),
            b'z' => Some(10),
            b'x' => Some(0),
            b'c' => Some(11),
            b'4' => Some(12),
            b'r' => Some(13),
            b'f' => Some(14),
            b'v' => Some(15),
            _ => None,
        };
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn cycle(&mut self) {
        if let Some(vx) = self.key_wait_target {
            if let Some(k) = self.key {
                self.registers[vx] = k;
                self.key_wait_target = None;
            }
        } else {
            self.rng.clock();
            self.fetch();
            self.exec();
        }
    }

    pub fn load_fonts(&mut self) {
        self.memory[..5 * 16].clone_from_slice(&[
            0xf0, 0x90, 0x90, 0x90, 0xf0, // 0
            0x10, 0x30, 0x10, 0x10, 0x10, // 1
            0xf0, 0x10, 0xf0, 0x80, 0xf0, // 2
            0xf0, 0x10, 0xf0, 0x10, 0xf0, // 3
            0x90, 0x90, 0xf0, 0x10, 0x10, // 4
            0xf0, 0x80, 0xf0, 0x10, 0xf0, // 5
            0xf0, 0x80, 0xf0, 0x90, 0xf0, // 6
            0xf0, 0x10, 0x10, 0x10, 0x10, // 7
            0xf0, 0x90, 0xf0, 0x90, 0xf0, // 8
            0xf0, 0x90, 0xf0, 0x10, 0xf0, // 9
            0xf0, 0x90, 0xf0, 0x90, 0x90, // A
            0x80, 0x80, 0xf0, 0x90, 0xf0, // b
            0xf0, 0x80, 0x80, 0x80, 0xf0, // C
            0x10, 0x10, 0xf0, 0x90, 0xf0, // d
            0xf0, 0x80, 0xe0, 0x80, 0xf0, // E
            0xf0, 0x80, 0xe0, 0x80, 0x80, // F
        ]);
    }

    pub fn load_game(&mut self, game: &[u8]) {
        let max_len = game.len().clamp(0, 0x1000 - 0x300) + 0x200;
        self.memory[0x200..max_len].clone_from_slice(game);
    }

    fn screen(&self) -> &[u8] {
        &self.memory[Self::DISPLAY_START..]
    }

    pub fn current_op(&self) -> u16 {
        self.op
    }
}

impl Default for CHIP8 {
    fn default() -> Self {
        let timer = Arc::new(AtomicU8::new(0));
        let o_t = timer.clone();
        Self {
            draw_flag: false,
            clear_flag: false,
            memory: [0; 0x1000],
            delay_timer: timer,
            i: 0x200,
            key: None,
            key_wait_target: None,
            op: 0,
            pc: 0x200,
            registers: [0; 16],
            rng: RNG(106), // just searched RNG on google, nothing more.
            sp: 0,
            stack: [0; 24],
            _thread: Some(std::thread::spawn(move || {
                thread::sleep(Duration::from_millis(17)); // ~16.6 period in 6 -> 17 approx
                o_t.fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |x| if x > 0 { Some(x - 1) } else { Some(x) },
                )
                .unwrap();
            })),
        }
    }
}
impl Drop for CHIP8 {
    fn drop(&mut self) {
        // stop the counting thread.
        let thread = self._thread.take().unwrap();
        thread.join().unwrap();
    }
}

struct RNG(u8);

impl RNG {
    // xor shift.
    pub fn clock(&mut self) {
        self.0 ^= self.0.wrapping_shl(13);
        self.0 ^= self.0.wrapping_shr(17);
        self.0 ^= self.0.wrapping_shl(5);
    }
}

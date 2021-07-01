use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    time::{Duration, Instant},
};

use chip8_interpreter::CHIP8;
use ncurses::*;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "chip8 interpreter",
    about = "a chip8 interpreter on your command line"
)]
struct Opt {
    /// File to use as game to run.
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    /// Customize color of output
    #[structopt(long = "color", default_value = "255")]
    svg_color: u8,
}

fn draw_pixel(w: WINDOW, x: i32, y: i32) {
    wmove(w, y + 1, x * 2 + 1);
    waddch(w, 32);
    waddch(w, 32);
}

fn set_pixel(w: WINDOW, x: i32, y: i32, on: bool) {
    wattrset(w, COLOR_PAIR(if on { 1 } else { 2 }));
    draw_pixel(w, x, y);
    wattroff(w, COLOR_PAIR(1));
}

struct InterpreterHandler {
    interpreter: CHIP8,
    // TODO: wrapper for ncurses window.
    // A wrapper for the ncurses window will enable me to
    // typedef window methods correctly so the compiler knows when the
    // window will be mutated.
    window: WINDOW,
}

trait Loop {
    /// Whenever a key is pressed the main loop
    /// will call this.
    fn register_key(&mut self, key: i32);
    /// Every loop.
    fn cycle(&mut self);
}

impl InterpreterHandler {
    pub fn new(window: WINDOW, game: &[u8]) -> Self {
        let mut interpreter = CHIP8::new();
        interpreter.load_fonts();
        interpreter.load_game(game);

        Self {
            interpreter,
            window,
        }
    }

    fn update_screen(&self) {
        for y in 0..32 {
            let line = self.interpreter.line_at(y);
            for x in 0..64 {
                let bit_value = line >> (63 - x) & 1;
                let bit_value = bit_value == 1;
                set_pixel(self.window, x as i32, y as i32, bit_value);
            }
        }
    }

    fn clear_screen(&self) {
        wclrtobot(self.window);
    }
}

impl Loop for InterpreterHandler {
    fn cycle(&mut self) {
        let mut updated = false;
        self.interpreter.cycle();
        if self.interpreter.clear_flag {
            self.clear_screen();
            self.interpreter.clear_flag = false;
            updated = true;
        }
        if self.interpreter.draw_flag {
            self.update_screen();
            self.interpreter.draw_flag = false;
            updated = true;
        }
        if updated {
            box_(self.window, 0, 0);
            wrefresh(self.window);
        }
    }

    fn register_key(&mut self, key: i32) {
        let ukey = (key & 0xff) as u8;
        self.interpreter.key(ukey);
    }
}

struct WithRate<L: Loop> {
    inner: L,
    target_frame: Duration,
    next_frame: Instant,
    window: WINDOW,
    key_buffer: VecDeque<i32>,
}

impl<L: Loop> Loop for WithRate<L> {
    #[inline(always)]
    fn register_key(&mut self, k: i32) {
        self.key_buffer.push_back(k);
    }

    fn cycle(&mut self) {
        let now = Instant::now();
        if self.next_frame > now {
            return;
        }
        if let Some(key) = self.key_buffer.pop_front() {
            self.inner.register_key(key);
        }
        self.inner.cycle();
        let elapsed = now.elapsed();

        self.display_metrics(elapsed);

        box_(self.window, 0, 0);
        wrefresh(self.window);

        self.next_frame =
            now + self.target_frame + self.target_frame.checked_sub(elapsed).unwrap_or_default();
    }
}

impl<L: Loop> WithRate<L> {
    pub fn new(window: WINDOW, target_frame: Duration, inner: L) -> Self {
        Self {
            window,
            target_frame,
            next_frame: Instant::now(),
            inner,
            key_buffer: VecDeque::new(),
        }
    }
    fn display_metrics(&mut self, elapsed: Duration) {
        let rt_elapsed = self.target_frame + elapsed;
        let (value, fmt, hertz) = {
            let micros = rt_elapsed.as_micros();
            if micros > 1000 {
                let millis = rt_elapsed.as_millis();
                (millis, "ms", 1000 / millis)
            } else {
                (micros, "us", 1000000 / micros)
            }
        };
        wclrtobot(self.window);
        wmove(self.window, 1, 1);
        waddstr(self.window, &format!("{} per tick: {} ({} Hz)", fmt, value, hertz));

        if elapsed > self.target_frame {
            waddstr(self.window, " !! falling behind !!");
        }


    }
}

fn main_loop(handles: &mut [&mut dyn Loop]) {
    noecho();
    nodelay(stdscr(), true);
    keypad(stdscr(), true);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    loop {
        let key = getch();
        if key != -1 {
            // escape
            if key == 27 {
                break;
            }
            for h in handles.iter_mut() {
                h.register_key(key);
            }
        }
        for h in handles.iter_mut() {
            h.cycle();
        }
    }
}

fn main() {
    let opts = Opt::from_args();
    let mut file = BufReader::new(File::open(opts.input_file).unwrap());
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    initscr();
    start_color();
    let (width, height) = {
        let mut x = 0;
        let mut y = 0;
        getmaxyx(stdscr(), &mut y, &mut x);
        (x as usize, y as usize)
    };

    init_pair(1, 0, opts.svg_color as i16);
    init_pair(2, 0, 0);

    let str = "Press ESC key to end the intepreter! (Press any key to start)";

    mvaddstr(1, 1, str);
    getch();
    clear();

    let interpreter_window = newwin(34, 130, (height / 2 - 17) as i32, (width / 2 - 63) as i32);
    let metrics_window = newwin(3, (width - 2) as i32, 1, 1);

    // 500Hz
    let target_duration = Duration::new(1, 0)
        .checked_div(500)
        .expect("failed when rhs != 0, what?");

    main_loop(&mut [&mut WithRate::new(
        metrics_window,
        target_duration,
        InterpreterHandler::new(interpreter_window, &buffer),
    )]);

    delwin(interpreter_window);
    delwin(metrics_window);
    endwin();
}

use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    thread::sleep,
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
        (x, y)
    };

    init_pair(1, 0, opts.svg_color as i16);
    init_pair(2, 0, 0);

    noecho();
    nodelay(stdscr(), true);
    keypad(stdscr(), true);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    let w = newwin(34, 130, height / 2 - 17, width / 2 - 63);

    let mut interpreter = CHIP8::new();
    interpreter.load_game(&buffer);
    interpreter.load_fonts();

    let target_duration = Duration::new(1, 0)
        .checked_div(500)
        .expect("failed when rhs != 0, what?");

    loop {
        let now = Instant::now();
        let key = getch();
        if key == 27 {
            break;
        }
        if key != -1 {
            interpreter.key(key as u8);
        }

        // println!("starting loop at {:?}", top_of_loop);

        interpreter.cycle();
        if interpreter.clear_flag {
            interpreter.clear_flag = false;
            // wclear(w);
        }
        if interpreter.draw_flag {
            interpreter.draw_flag = false;

            for y in 0..32 {
                let line = interpreter.line_at(y);
                for x in 0..64 {
                    let bit_value = line >> (63 - x) & 1;
                    let bit_value = if bit_value == 1 { true } else { false };
                    set_pixel(w, x as i32, y as i32, bit_value);
                }
            }
            // debug_assert!(y == 32 && x == 0, "Didn't do screen swipe well.");
        }

        box_(w, 0, 0);
        wrefresh(w);

        let elapsed = now.elapsed();
        if let Some(remaining) = target_duration.checked_sub(elapsed) {
            sleep(remaining);
        }
    }

    delwin(w);
    endwin();
}

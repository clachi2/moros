use crate::api::font::Font;
use crate::api::fs::write;
use crate::api::process::ExitCode;
use crate::sys::console;
use crate::sys::mouse::{get_event_buffer};
use crate::sys::vga::framebuffer;

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
struct Rectangle {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u8,
}

impl Rectangle {
    fn new(x: usize, y: usize, width: usize, height: usize, color: u8) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color,
        }
    }

    fn move_left(&mut self) {
        if self.x > 0 {
            self.x -= 5;
        }
    }

    fn move_right(&mut self) {
        if self.x + self.width < WIDTH {
            self.x += 5;
        }
    }

    fn move_up(&mut self) {
        if self.y > 0 {
            self.y -= 5;
        }
    }

    fn move_down(&mut self) {
        if self.y + self.height < HEIGHT {
            self.y += 5;
        }
    }

    fn set_color(&mut self, color: u8) {
        self.color = color;
    }

    fn add_pos(&mut self, x: i8, y: i8) {
        if (self.x as i32 + x as i32) < 0 {
            self.x = 0;
        } else if (self.x as i32 + x as i32)  > WIDTH as i32 {
            self.x = WIDTH - 1;
        } else {
            self.x += x as usize;
        }
        if (self.y as i32 + y as i32) < 0 {
            self.y = 0;
        } else if (self.y as i32 + y as i32) > HEIGHT as i32 {
            self.y = HEIGHT - 1;
        } else {
            self.y += y as usize;
        }
    }
}

struct Framebuffer {
    buffer: [u8; WIDTH * HEIGHT],
}

impl Framebuffer {
    fn new() -> Self {
        Self {
            buffer: [0; WIDTH * HEIGHT],
        }
    }

    fn clear(&mut self) {
        self.buffer = [0; WIDTH * HEIGHT];
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: u8) {
        if x < WIDTH && y < HEIGHT {
            self.buffer[y * WIDTH + x] = color;
        }
    }

    fn draw_rectangle(&mut self, rect: &Rectangle) {
        for dy in 0..rect.height {
            for dx in 0..rect.width {
                self.set_pixel(rect.x + dx, rect.y + dy, rect.color);
            }
        }
    }
}

pub fn main(args: &[&str]) -> Result<(), ExitCode> {

    kprintln!("Starting game...");


    write("/dev/vga/mode", b"320x200").expect("Could not switch to graphics mode");
    print!("\x1b[?25l"); // Cursor ausblenden

    kprintln!("Framebuffer resolution set to 320x200");

    // let mut fb = Framebuffer::new();
    let mut fb = framebuffer::Framebuffer::new(320, 200, 8, "/dev/vga/buffer");
    let mut rect = Rectangle::new(WIDTH / 2 - 20, HEIGHT / 2 - 20, 10, 10, 0x3);

    kprintln!("Rectangle and framebuffer initialized");

    let buf = include_bytes!("../../dsk/ini/fonts/cp857-8x8.psf");
    let font = Font::try_from(&buf[..]).unwrap();

    kprintln!("Font loaded");

    get_event_buffer().clear_events();

    kprintln!("Mouse buffer cleared");

    // Hauptspielschleife
    loop {
        // Exit-Bedingungen prüfen
        if console::end_of_text() || console::end_of_transmission() {
            break;
        }

        let mut x_pos = 0;
        let mut y_pos = 0;

        while let Some(event) = get_event_buffer().get_last_event() {
            x_pos += event.x_movement;
            y_pos += event.y_movement;
            if event.is_left_click() {
                rect.set_color(0x4);
            } else {
                rect.set_color(0x3);
            }
        }

        rect.add_pos(x_pos, y_pos);

        // Tastatureingaben verarbeiten - non-blocking
        console::disable_echo();
        console::enable_raw();
        {
            let mut stdin = console::STDIN.lock();
            if !stdin.is_empty() {
                match stdin.remove(0) {
                    'q' => break,
                    'w' => rect.move_up(),
                    's' => rect.move_down(),
                    'a' => rect.move_left(),
                    'd' => rect.move_right(),
                    _ => {}
                }
            }
        }
        console::enable_echo();
        console::disable_raw();

        // Rendern
        // fb.clear();
        // fb.draw_rectangle(&rect);
        //
        // write("/dev/vga/buffer", &fb.buffer).expect("Could not write to buffer");

        fb.clear();
        fb.draw_rectangle(rect.x, rect.y, rect.width, rect.height, rect.color);
        fb.draw_line(10,10, 100, 200, 0x1);
        fb.draw_line(100, 10, 10, 200, 0x2);
        fb.draw_circle(150, 100, 50, 0x5, true);
        fb.draw_circle(200, 100, 50, 0x6, false);

        fb.draw_text(10, 10, "Hallo Welt!", 0x7, &font, 1.0);
        fb.flush();

        // Frame-Rate Kontrolle
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }

    // Aufräumen
    print!("\x1b[?25h");
    write("/dev/vga/mode", b"80x25").expect("Could not return to text mode");

    Ok(())
}

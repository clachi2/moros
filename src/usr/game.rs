use crate::api::fs::{write};
use crate::api::process::ExitCode;
use crate::sys::console;
use crate::sys::mouse::MOUSE_BUFFER;

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
    write("/dev/vga/mode", b"320x200").expect("Could not switch to graphics mode");
    print!("\x1b[?25l"); // Cursor ausblenden

    let mut fb = Framebuffer::new();
    let mut rect = Rectangle::new(WIDTH / 2 - 20, HEIGHT / 2 - 20, 40, 40, 0x3);

    MOUSE_BUFFER.get().unwrap().clear_events();
    // Hauptspielschleife
    loop {
        // Exit-Bedingungen prüfen
        if console::end_of_text() || console::end_of_transmission() {
            break;
        }

        let mut x_pos = 0;
        let mut y_pos = 0;

        while let Some(event) = MOUSE_BUFFER.get().unwrap().get_last_event() {
            x_pos += event.x_movement;
            y_pos += event.y_movement;
        }

        if x_pos > 0 {
            rect.move_right();
        } else if x_pos < 0 {
            rect.move_left();
        }
        if y_pos > 0 {
            rect.move_down();
        } else if y_pos < 0 {
            rect.move_up();
        }

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
        fb.clear();
        fb.draw_rectangle(&rect);

        write("/dev/vga/buffer", &fb.buffer).expect("Could not write to buffer");

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

use crate::api::fs::write;
use crate::sys;
use crate::sys::vga::framebuffer;
use crate::usr::game::state::Map;
use alloc::vec;
use alloc::vec::Vec;

pub(crate) struct Renderer {
    screen_width: usize,
    screen_height: usize,
    color_depth: u8,
    framebuffer: framebuffer::Framebuffer,
    tile_size: usize,
}

impl Renderer {
    pub fn new(screen_width: usize, screen_height: usize, color_depth: u8, tile_size: usize) -> Self {
        let framebuffer =
            framebuffer::Framebuffer::new(screen_width, screen_height, 8, "/dev/vga/buffer");

        Renderer {
            screen_width,
            screen_height,
            color_depth,
            framebuffer,
            tile_size,
        }
    }

    pub fn init(&mut self) {
        write("/dev/vga/mode", b"320x200").expect("Could not switch to graphics mode");
        print!("\x1b[?25l"); // Cursor ausblenden
        // Clear the framebuffer
        self.framebuffer.clear();
    }

    pub fn draw(&mut self) {

        self.framebuffer.draw_line(0, 0, 320, 200, 0x3); // Example line for testing



        //draw 255 rainbow
        for y in 0..self.screen_height {
            for x in 0..255 {
                self.framebuffer.draw_pixel(x, y, x as u8);
            }
        }

        // flush framebuffer to file
        self.framebuffer.flush();

    }

    pub fn draw_map(&mut self, map: Map) {
        // Clear the map buffer
        self.framebuffer.clear();

        for y in 0..map.tiles_y {
            for x in 0..map.tiles_x {
                if map.vertical_walls[y * map.tiles_x + x] {
                    let screen_x = x * self.tile_size;
                    let screen_y = y * self.tile_size;

                    // Draw a vertical line on the left side of the tile
                    for dy in 0..self.tile_size {
                        self.framebuffer.draw_pixel(screen_x, screen_y + dy, 0xE);
                    }
                }
                if map.horizontal_walls[y * map.tiles_x + x] {
                    let screen_x = x * self.tile_size;
                    let screen_y = y * self.tile_size;

                    // Draw a horizontal line on the top of the tile
                    for dx in 0..self.tile_size {
                        self.framebuffer.draw_pixel(screen_x + dx, screen_y, 0xE);
                    }
                }
            }
        }
    }
}

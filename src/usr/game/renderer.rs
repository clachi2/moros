use crate::api::fs::write;
use crate::sys;
use crate::sys::vga::{VgaPalette, framebuffer};
use crate::usr::game::state::{Direction, Map, GUI_WIDTH, PLAYER_SIZE};
use alloc::vec;
use alloc::vec::Vec;

pub(crate) struct Renderer {
    screen_width: usize,
    screen_height: usize,
    color_depth: u8,
    framebuffer: framebuffer::Framebuffer,
    map_buffer: framebuffer::Framebuffer,
    tiles_x: usize,
    tiles_y: usize,
    tile_size: usize,
}

impl Renderer {
    pub fn new(
        screen_width: usize,
        screen_height: usize,
        color_depth: u8,
        tiles_x: usize,
        tiles_y: usize,
        tile_size: usize,
    ) -> Self {
        let framebuffer =
            framebuffer::Framebuffer::new(screen_width, screen_height, 8, "/dev/vga/buffer");
        let map_buffer =
            framebuffer::Framebuffer::new(tiles_x * tile_size, tiles_y * tile_size, 8, "");

        Renderer {
            screen_width,
            screen_height,
            color_depth,
            framebuffer,
            map_buffer,
            tiles_x,
            tiles_y,
            tile_size,
        }
    }

    pub fn init(&mut self) {
        if self.screen_width == 320 && self.screen_height == 200 {
            write("/dev/vga/mode", b"320x200").expect("Could not switch to graphics mode");
            print!("\x1b[?25l"); // Cursor ausblenden
        } else {
            panic!("Unsupported screen resolution for game");
        }
        //set color palette
        VgaPalette::vga_256().write();
        // Clear the framebuffer
        self.framebuffer.clear();
    }

    pub fn flush(&mut self) {
        // flush framebuffer to file
        self.framebuffer.flush();
    }

    pub fn draw_player(&mut self, x: usize, y: usize, color: u8, direction: Direction) {
        for dy in 0..PLAYER_SIZE {
            for dx in 0..PLAYER_SIZE {
                self.map_buffer
                    .draw_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_map(&mut self) {
        // write map to framebuffer
        self.framebuffer
            .copy_from_at(&self.map_buffer, GUI_WIDTH, 0);
    }

    pub fn draw_map_buffer(&mut self, map: Map) {
        // Clear the map buffer
        self.map_buffer.clear();

        // Draw the map tiles
        for y in 0..map.tiles_y {
            for x in 0..map.tiles_x {
                let tile = map.tiles[y * map.tiles_x + x].clone();
                if tile.up {
                    self.map_buffer.draw_line(
                        x * self.tile_size,
                        y * self.tile_size,
                        (x + 1) * self.tile_size - 1,
                        y * self.tile_size,
                        0x0f, // White color
                    );
                }
                if tile.right {
                    self.map_buffer.draw_line(
                        (x + 1) * self.tile_size - 1,
                        y * self.tile_size,
                        (x + 1) * self.tile_size - 1,
                        (y + 1) * self.tile_size - 1,
                        0x0f, // White color
                    );
                }
                if tile.down {
                    self.map_buffer.draw_line(
                        x * self.tile_size,
                        (y + 1) * self.tile_size - 1,
                        (x + 1) * self.tile_size - 1,
                        (y + 1) * self.tile_size - 1,
                        0x0f, // White color
                    );
                }
                if tile.left {
                    self.map_buffer.draw_line(
                        x * self.tile_size,
                        y * self.tile_size,
                        x * self.tile_size,
                        (y + 1) * self.tile_size - 1,
                        0x0f, // White color
                    );
                }
            }
        }
    }
}

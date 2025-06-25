use crate::api::fs::write;
use crate::sys::vga::{VgaPalette, framebuffer};
use crate::usr::game::state::{GUI_WIDTH, Map, PLAYER_SIZE};

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

    pub fn deinit(&mut self) {
        write("/dev/vga/mode", b"80x25").expect("Could not switch to graphics mode");
        VgaPalette::default().write();
        print!("\x1b[?25h"); // Cursor einblenden
    }

    pub fn flush(&mut self) {
        // flush framebuffer to file
        self.framebuffer.flush();
    }

    pub fn draw_player(&mut self, x: isize, y: isize, color: u8) {
        for dy in 0..PLAYER_SIZE {
            for dx in 0..PLAYER_SIZE {
                self.framebuffer
                    .draw_pixel(x + dx as isize, y + dy as isize, color);
            }
        }
    }

    pub fn draw_mouse_cursor(&mut self, x: isize, y: isize) {
        // Draw a simple crosshair as mouse cursor
        let cursor_color = 0x0f; // White color
        self.framebuffer.draw_line(x - 4, y, x + 4, y, cursor_color);
        self.framebuffer.draw_line(x, y - 4, x, y + 4, cursor_color);
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
                        (x * self.tile_size) as isize,
                        (y * self.tile_size) as isize,
                        ((x + 1) * self.tile_size - 1) as isize,
                        (y * self.tile_size) as isize,
                        0x0f, // White color
                    );
                }
                if tile.right {
                    self.map_buffer.draw_line(
                        ((x + 1) * self.tile_size - 1) as isize,
                        (y * self.tile_size) as isize,
                        ((x + 1) * self.tile_size - 1) as isize,
                        ((y + 1) * self.tile_size - 1) as isize,
                        0x0f, // White color
                    );
                }
                if tile.down {
                    self.map_buffer.draw_line(
                        (x * self.tile_size) as isize,
                        ((y + 1) * self.tile_size - 1) as isize,
                        ((x + 1) * self.tile_size - 1) as isize,
                        ((y + 1) * self.tile_size - 1) as isize,
                        0x0f, // White color
                    );
                }
                if tile.left {
                    self.map_buffer.draw_line(
                        (x * self.tile_size) as isize,
                        (y * self.tile_size) as isize,
                        (x * self.tile_size) as isize,
                        ((y + 1) * self.tile_size - 1) as isize,
                        0x0f, // White color
                    );
                }
            }
        }
    }
}

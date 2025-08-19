use crate::api::font::Font;
use crate::api::fs::write;
use crate::sys::vga::{VgaPalette, framebuffer};
use crate::usr::game::tanks::map::Map;
use crate::usr::game::tanks::state::{GUI_HEIGHT_PER_PLAYER, GUI_WIDTH, MAX_AMMO, PLAYER_SIZE};
use alloc::format;
use alloc::vec::Vec;

pub enum Color {
    Black = 0x00,
    Blue = 0x01,
    Green = 0x02,
    Cyan = 0x03,
    Red = 0x04,
    Magenta = 0x05,
    Brown = 0x06,
    LightGray = 0x07,
    DarkGray = 0x08,
    LightBlue = 0x09,
    LightGreen = 0x0A,
    LightCyan = 0x0B,
    LightRed = 0x0C,
    LightMagenta = 0x0D,
    Yellow = 0x0E,
    White = 0x0F,
}

pub struct Renderer {
    screen_width: usize,
    screen_height: usize,
    color_depth: u8,
    pub framebuffer: framebuffer::Framebuffer,
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
        self.framebuffer.clear(Color::LightGray as u8);
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

    pub fn clear(&mut self) {
        self.framebuffer.clear(Color::LightGray as u8);
    }

    pub fn draw_player(&mut self, x: isize, y: isize, color: u8) {
        for dy in 0..PLAYER_SIZE {
            for dx in 0..PLAYER_SIZE {
                self.framebuffer
                    .draw_pixel(x + dx as isize, y + dy as isize, color);
            }
        }
    }

    pub fn draw_bullet(&mut self, x: isize, y: isize, color: u8) {
        // Draw a simple bullet as a small square
        for dy in 0..2 {
            for dx in 0..2 {
                self.framebuffer
                    .draw_pixel(x + dx as isize, y + dy as isize, color);
            }
        }
    }

    pub fn draw_stat(&mut self, index: usize, player_id: usize, points: usize, ammo: usize) {
        // Draw 2px outline around the stats
        let y = (index * GUI_HEIGHT_PER_PLAYER) as isize;
        self.framebuffer.draw_rectangle(
            0,
            y,
            GUI_WIDTH as isize,
            GUI_HEIGHT_PER_PLAYER as isize,
            player_id as u8, // Use player_id as color
        );
        self.framebuffer.draw_rectangle(
            0 + 2,
            y + 2,
            GUI_WIDTH as isize - 4,
            GUI_HEIGHT_PER_PLAYER as isize - 4,
            Color::LightGray as u8, // Use player_id as color
        );

        // Draw points
        let points_text = format!("P:{}", points);
        let buf = include_bytes!("../../../../dsk/ini/fonts/cp857-8x8.psf");
        let font = Font::try_from(&buf[..]).unwrap();
        self.framebuffer.draw_text(
            4,
            y + 6,
            &points_text,
            Color::Black as u8, // Black color for text
            &font,
            1.0,
        );

        let bullet_bitmap = [
            false, true, true, false, true, false, false, true, true, false, false, true, true,
            false, false, true, true, false, false, true, true, false, false, true, true, false,
            false, true, true, false, false, true, true, false, false, true, true, false, false,
            true, true, false, false, true, true, false, false, true, true, true, true, true,
        ];

        for i in 0..MAX_AMMO {
            let bullet_x = GUI_WIDTH as isize - (MAX_AMMO as isize * 5) + (i * 5) as isize - 2;
            let bullet_y = y + 3;
            self.framebuffer.draw_bitmap(
                bullet_x,
                bullet_y,
                &bullet_bitmap,
                4,
                13,
                Color::Black as u8,
                Color::LightGray as u8,
            );
            if i < ammo {
                // filled ammo
                self.framebuffer
                    .draw_rectangle(bullet_x + 1, bullet_y + 1, 2, 11, player_id as u8)
            }
        }
    }

    pub fn draw_mouse_cursor(&mut self, x: isize, y: isize, color: u8) {
        // Draw a simple crosshair as mouse cursor
        //let cursor_color = Color::Red as u8; // White color
        self.framebuffer.draw_line(x - 4, y, x + 4, y, color);
        self.framebuffer.draw_line(x, y - 4, x, y + 4, color);
    }

    pub fn draw_areas(&mut self, areas: &Vec<Vec<(usize, usize)>>) {
        // Draw areas on the map buffer
        for (i, area) in areas.iter().enumerate() {
            for &(x, y) in area {
                let x_pos = x * self.tile_size;
                let y_pos = y * self.tile_size;
                self.framebuffer.draw_rectangle(
                    (x_pos + GUI_WIDTH + 7) as isize,
                    (y_pos + 7) as isize,
                    (self.tile_size - 14) as isize,
                    (self.tile_size - 14) as isize,
                    (i + 1) as u8,
                );
            }
        }
    }

    pub fn draw_all_colors(&mut self) {
        for i in 0..256 {
            self.framebuffer.draw_line(
                i,
                0,
                i,
                (self.screen_height - 1) as isize,
                i as u8, // Use color index as color
            );
        }
    }

    pub fn draw_map(&mut self) {
        // write map to framebuffer
        self.framebuffer
            .copy_from_at(&self.map_buffer, GUI_WIDTH, 0);
    }

    pub fn draw_map_buffer(&mut self, map: Map) {
        // Clear the map buffer
        self.map_buffer.clear(Color::LightGray as u8);

        // Draw the map tiles
        for y in 0..map.tiles_y {
            for x in 0..map.tiles_x {
                let tile = map.tiles[y * map.tiles_x + x].clone();
                let mut line;
                let x_i = x as isize;
                let y_i = y as isize;
                if tile.up {
                    line = map.wall_coords(x_i, y_i, 0);
                    self.map_buffer
                        .draw_line(line.0, line.1, line.2, line.3, Color::Black as u8);
                }
                if tile.right {
                    line = map.wall_coords(x_i, y_i, 1);
                    self.map_buffer
                        .draw_line(line.0, line.1, line.2, line.3, Color::Black as u8);
                }
                if tile.down {
                    line = map.wall_coords(x_i, y_i, 2);
                    self.map_buffer
                        .draw_line(line.0, line.1, line.2, line.3, Color::Black as u8);
                }
                if tile.left {
                    line = map.wall_coords(x_i, y_i, 3);
                    self.map_buffer
                        .draw_line(line.0, line.1, line.2, line.3, Color::Black as u8);
                }
            }
        }
    }
}

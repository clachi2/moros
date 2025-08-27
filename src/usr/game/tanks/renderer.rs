use crate::api::font::Font;
use crate::api::fs::write;
use crate::sys::vga::{VgaPalette, framebuffer};
use crate::usr::game::tanks::map::Map;
use crate::usr::game::tanks::state::{GUI_HEIGHT_PER_PLAYER, GUI_WIDTH, MAX_AMMO, PLAYER_SIZE};
use alloc::format;
use alloc::vec::Vec;

/// Basic colors for 8-bit color depth
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

/// Renderer for drawing game elements to the screen using a framebuffer.
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
    /// Creates a new `Renderer` instance with the specified screen dimensions, color depth, and tile configuration.
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

    /// Initializes the game environment by configuring the display settings (especially by changing to 320x200 VGA-mode), setting the color palette,
    /// and clearing the framebuffer.
    ///
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

    /// Deinitializes the VGA setup and restores the default VGA mode and settings.
    ///
    pub fn deinit(&mut self) {
        write("/dev/vga/mode", b"80x25").expect("Could not switch to graphics mode");
        VgaPalette::default().write();
        print!("\x1b[?25h"); // Cursor einblenden
    }

    /// Flushes the current framebuffer content to the display.
    pub fn flush(&mut self) {
        // flush framebuffer to file
        self.framebuffer.flush();
    }

    /// Clears the framebuffer with a light gray color.
    pub fn clear(&mut self) {
        self.framebuffer.clear(Color::LightGray as u8);
    }

    /// Draws the player character on the screen.
    ///
    /// This method renders a square-shaped player of a predefined size (`PLAYER_SIZE`)
    /// at the specified `(x, y)` coordinates on the framebuffer. Each pixel in the square
    /// is drawn with the provided color value.
    pub fn draw_player(&mut self, x: isize, y: isize, color: u8) {
        for dy in 0..PLAYER_SIZE {
            for dx in 0..PLAYER_SIZE {
                self.framebuffer
                    .draw_pixel(x + dx as isize, y + dy as isize, color);
            }
        }
    }

    /// Draws a bullet on the screen at the specified (x, y) coordinates.
    ///
    /// The bullet is rendered as a 2x2 square of pixels and is drawn using the specified color.
    /// This function modifies the internal framebuffer to render the bullet.
    ///
    pub fn draw_bullet(&mut self, x: isize, y: isize, color: u8) {
        // Draw a simple bullet as a small square
        for dy in 0..2 {
            for dx in 0..2 {
                self.framebuffer
                    .draw_pixel(x + dx as isize, y + dy as isize, color);
            }
        }
    }

    /// Draws a player's statistics on the GUI, including a border, player's points, and remaining ammo.
    ///
    /// # Parameters
    /// - `index`: The player's index (used for determining the Y position of the stat display).
    /// - `player_id`: A unique identifier for the player, used to determine the color of elements.
    /// - `points`: The number of points currently scored by the player.
    /// - `ammo`: The amount of ammo remaining for the player.
    ///
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

        // Draw ammo
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

    /// Draws a mouse cursor at the specified (x, y) coordinates using the given color.
    pub fn draw_mouse_cursor(&mut self, x: isize, y: isize, color: u8) {
        // Draw a simple crosshair as mouse cursor
        self.framebuffer.draw_line(x - 4, y, x + 4, y, color);
        self.framebuffer.draw_line(x, y - 4, x, y + 4, color);
    }

    /// Draws marked areas on a map buffer. (This is for the map-demo only)
    ///
    /// This method iterates over the provided areas, where each area consists of a
    /// vector of coordinate pairs `(usize, usize)` representing `(x, y)` positions on
    /// the map. For each coordinate, a rectangle is drawn on a framebuffer at a specified
    /// position, scaled and offset by the tile size and GUI dimensions.
    ///
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

    /// Copies the map buffer to the main framebuffer at a fixed position.
    pub fn draw_map(&mut self) {
        // write map to framebuffer
        self.framebuffer
            .copy_from_at(&self.map_buffer, GUI_WIDTH, 0);
    }

    /// Draws the map layout onto the map buffer.
    ///
    /// This function takes a `Map` object and renders its tile-based representation
    /// onto the `map_buffer`. It first clears the buffer with a light gray background,
    /// then iterates over the tiles of the `map` and draws the walls (if present)
    /// for each tile in black.
    ///
    /// # Parameters
    /// - `map`: The `Map` structure representing the tile-based layout to be drawn,
    ///   which includes tile information (e.g., wall presence) and dimensions.
    ///
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

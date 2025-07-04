use crate::api::font::Font;
use crate::api::fs::write;
use alloc::vec;
use alloc::vec::Vec;

pub struct Framebuffer {
    width: usize,
    height: usize,
    color_depth: usize,
    pitch: usize,
    file_path: &'static str,
    internal_buffer: Vec<u8>,
    buffer_size: usize,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize, color_depth: usize, file_path: &'static str) -> Self {
        let pitch = width * (color_depth / 8);
        let buffer_size: usize = height * pitch;
        let internal_buffer = vec![0u8; buffer_size];

        Framebuffer {
            width,
            height,
            color_depth,
            pitch,
            file_path,
            internal_buffer,
            buffer_size,
        }
    }

    pub fn copy_from(&mut self, other: &Framebuffer) {
        if self.width != other.width
            || self.height != other.height
            || self.color_depth != other.color_depth
        {
            panic!("Cannot copy from framebuffer with different dimensions or color depth");
        }
        self.internal_buffer.copy_from_slice(&other.internal_buffer);
    }

    pub fn copy_from_at(&mut self, other: &Framebuffer, x: usize, y: usize) {
        if self.width < x + other.width || self.height < y + other.height {
            panic!("Cannot copy from framebuffer at position outside of bounds");
        }
        for dy in 0..other.height {
            for dx in 0..other.width {
                let src_index = dy * other.pitch + dx * (other.color_depth / 8);
                let dest_index = (y + dy) * self.pitch + (x + dx) * (self.color_depth / 8);
                self.internal_buffer[dest_index] = other.internal_buffer[src_index];
            }
        }
    }

    pub fn get_buffer(&self) -> &Vec<u8> {
        &self.internal_buffer
    }

    pub fn clear(&mut self, color: u8) {
        for byte in self.internal_buffer.iter_mut() {
            *byte = color;
        }
    }

    pub fn flush(&self) {
        write(self.file_path, &self.internal_buffer).expect("could not write to framebuffer file");
    }

    pub fn draw_pixel(&mut self, x: isize, y: isize, color: u8) {
        if x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize {
            let x = x as usize;
            let y = y as usize;
            let index = y * self.pitch + x * (self.color_depth / 8);
            self.internal_buffer[index] = color;
        }
    }

    pub fn draw_line(&mut self, mut x1: isize, mut y1: isize, mut x2: isize, mut y2: isize, color: u8) {
        if x1 >= self.width as isize {
            x1 = self.width as isize - 1;
        }
        if y1 >= self.height as isize {
            y1 = self.height as isize - 1;
        }
        if x2 >= self.width as isize {
            x2 = self.width as isize - 1;
        }
        if y2 >= self.height as isize {
            y2 = self.height as isize - 1;
        }

        let dx = (x2- x1).abs();
        let dy = (y2- y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x1;
        let mut y = y1;

        loop {
            self.draw_pixel(x, y, color);
            if x == x2 && y == y2 {
                break;
            }
            let err2 = err * 2;
            if err2 > -dy {
                err -= dy;
                x = x + sx;
            }
            if err2 < dx {
                err += dx;
                y = y + sy;
            }
        }
    }

    pub fn draw_rectangle(&mut self, x: isize, y: isize, width: isize, height: isize, color: u8) {
        for dy in 0..height {
            for dx in 0..width {
                self.draw_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_circle(&mut self, cx: isize, cy: isize, radius: usize, color: u8, filled: bool) {
        let mut x = radius as isize;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            if filled {
                self.draw_line(cx - x, cy + y, cx + x, cy + y, color);
                self.draw_line(cx - x, cy - y, cx + x, cy - y, color);
                self.draw_line(cx - y, cy + x, cx + y, cy + x, color);
                self.draw_line(cx - y, cy - x, cx + y, cy - x, color);
            } else {
                self.draw_pixel(cx + x, cy + y, color);
                self.draw_pixel(cx - x, cy + y, color);
                self.draw_pixel(cx + y, cy + x, color);
                self.draw_pixel(cx - y, cy + x, color);
                self.draw_pixel(cx + x, cy - y, color);
                self.draw_pixel(cx - x, cy - y, color);
                self.draw_pixel(cx + y, cy - x, color);
                self.draw_pixel(cx - y, cy - x, color);
            }

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    pub fn draw_text(
        &mut self,
        x: isize,
        y: isize,
        text: &str,
        color: u8,
        font: &Font,
        scale: f32, // z. B. 0.5 für halb, 1.0 für normal
    ) {
        let mut cursor_x = x as f32;

        for c in text.chars() {
            let char_index = c as usize;
            if char_index >= font.size as usize {
                continue;
            }

            let offset = char_index * font.height as usize;
            for dy in 0..font.height as usize {
                let row = font.data[offset + dy];
                for dx in 0..8 {
                    if (row >> (7 - dx)) & 1 == 1 {
                        let pixel_x = (cursor_x + dx as f32 * scale) as isize;
                        let pixel_y = (y as f32 + dy as f32 * scale) as isize;

                        self.draw_pixel(pixel_x, pixel_y, color);
                    }
                }
            }

            cursor_x += 8.0 * scale;
        }
    }

    pub fn draw_bitmap(
        &mut self,
        x: isize,
        y: isize,
        bitmap: &[bool],
        width: usize,
        height: usize,
        fg: u8,
        bg: u8,
    ) {
        for dy in 0..height {
            for dx in 0..width {
                if bitmap[dy * width + dx] {
                    self.draw_pixel(x + dx as isize, y + dy as isize, fg);
                } else {
                    self.draw_pixel(x + dx as isize, y + dy as isize, bg);
                }
            }
        }
    }
}

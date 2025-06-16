use alloc::vec;
use alloc::vec::Vec;

pub(crate) struct Map {
    pub(crate) tiles_x: usize,
    pub(crate) tiles_y: usize,
    pub(crate) vertical_walls: Vec<bool>,
    pub(crate) horizontal_walls: Vec<bool>,
}

impl Map {
    pub fn new(tiles_x: usize, tiles_y: usize) -> Self {
        let vertical_walls = vec![false; tiles_x * tiles_y];
        let horizontal_walls = vec![false; tiles_x * tiles_y];

        Map {
            tiles_x,
            tiles_y,
            vertical_walls,
            horizontal_walls,
        }
    }

    pub fn set_vertical_wall(&mut self, x: usize, y: usize, value: bool) {
        if x < self.tiles_x && y < self.tiles_y {
            self.vertical_walls[y * self.tiles_x + x] = value;
        }
    }

    pub fn set_horizontal_wall(&mut self, x: usize, y: usize, value: bool) {
        if x < self.tiles_x && y < self.tiles_y {
            self.horizontal_walls[y * self.tiles_x + x] = value;
        }
    }

    pub fn set_outer_walls(&mut self) {
        for x in 0..self.tiles_x {
            self.set_vertical_wall(0, x, true);
            self.set_vertical_wall(self.tiles_y - 1, x, true);
        }
        for y in 0..self.tiles_y {
            self.set_horizontal_wall(y, 0, true);
            self.set_horizontal_wall(y, self.tiles_x -1, true);
        }
    }

    pub fn set_vertical_walls_bitmap(&mut self, bitmap: &[bool]) {
        kprintln!("{} {}", bitmap.len(), self.vertical_walls.len());
         // Ensure the bitmap length matches the vertical walls length
         // This is a simple check to avoid panic in case of mismatched lengths
         // In a real application, you might want to handle this more gracefully
        if bitmap.len() == self.vertical_walls.len() {
            self.vertical_walls.copy_from_slice(bitmap);
        } else {
            panic!("Bitmap length does not match vertical walls length");
        }
    }

    pub fn set_horizontal_walls_bitmap(&mut self, bitmap: &[bool]) {
        kprintln!("{} {}", bitmap.len(), self.horizontal_walls.len());
        if bitmap.len() == self.horizontal_walls.len() {
            self.horizontal_walls.copy_from_slice(bitmap);
        } else {
            panic!("Bitmap length does not match horizontal walls length");
        }
    }

    pub fn kprint_vec(&self) {
        kprintln!("Vertical Walls:");
        for y in 0..self.tiles_y {
            for x in 0..self.tiles_x {
                let vertical_wall = self.vertical_walls[y * self.tiles_x + x];
                if vertical_wall {
                    kprint!("1");
                } else {
                    kprint!("0");
                }
            }
            kprintln!("");
        }
        kprintln!("Horizontal Walls:");
        for y in 0..self.tiles_y {
            for x in 0..self.tiles_x {
                let horizontal_wall = self.horizontal_walls[y * self.tiles_x + x];
                if horizontal_wall {
                    kprint!("1");
                } else {
                    kprint!("0");
                }
            }
            kprintln!("");
        }
    }

    pub fn auto_set_inner_walls(&mut self) {
        // TODO: Implement random wall generation logic
    }
}

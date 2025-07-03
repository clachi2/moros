use crate::sys::rng::get_u64;
use crate::usr::game::state::{PLAYER_SIZE, WALL_DENSITY};
use alloc::vec;
use alloc::vec::Vec;


pub(crate) struct Direction {
    pub(crate) up: bool,
    pub(crate) right: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
}

impl Clone for Direction {
    fn clone(&self) -> Self {
        Direction {
            up: self.up,
            right: self.right,
            down: self.down,
            left: self.left,
        }
    }
}

pub(crate) struct Map {
    pub(crate) size_x: usize,
    pub(crate) size_y: usize,
    pub(crate) tiles_x: usize,
    pub(crate) tiles_y: usize,
    pub(crate) tile_size: usize, // Size of each tile in pixels
    pub(crate) tiles: Vec<Direction>,
}

impl Clone for Map {
    fn clone(&self) -> Self {
        Map {
            size_x: self.size_x,
            size_y: self.size_y,
            tiles_x: self.tiles_x,
            tiles_y: self.tiles_y,
            tile_size: self.tile_size,
            tiles: self.tiles.clone(),
        }
    }
}

impl Map {
    pub fn new(
        size_x: usize,
        size_y: usize,
        tiles_x: usize,
        tiles_y: usize,
        tile_size: usize,
    ) -> Self {
        let tile_walls = vec![
            Direction {
                up: false,
                down: false,
                left: false,
                right: false
            };
            tiles_x * tiles_y
        ];
        Map {
            size_x,
            size_y,
            tiles_x,
            tiles_y,
            tile_size,
            tiles: tile_walls,
        }
    }

    pub fn wall_coords(&self, x: isize, y: isize, facing: isize) -> (isize, isize, isize, isize) {
        // facing: 0 = up, 1 = right, 2 = down, 3 = left
        let tx = self.tiles_x as isize;
        let ty = self.tiles_y as isize;
        let ts = self.tile_size as isize;
        if x >= tx || y >= ty {
            panic!("Tile coordinates out of bounds");
        }
        match facing {
            0 => {
                // (x * ts, y * ts, (x + 1) * ts - 1, y * ts) // up
                (x * ts, y * ts, (x + 1) * ts, y * ts) // up
            }
            1 => {
                // ((x + 1) * ts - 1, y * ts, (x + 1) * ts - 1, (y + 1) * ts - 1) // right
                ((x + 1) * ts - 1, y * ts, (x + 1) * ts - 1, (y + 1) * ts) // right
            }
            2 => {
                // (x * ts, (y + 1) * ts - 1, (x + 1) * ts - 1, (y + 1) * ts - 1) // down
                (x * ts, (y + 1) * ts - 1, (x + 1) * ts, (y + 1) * ts - 1) // down
            }
            3 => {
                // (x * ts, y * ts, x * ts, (y + 1) * ts - 1) // left
                (x * ts, y * ts, x * ts, (y + 1) * ts) // left
            }
            _ => panic!("Invalid facing direction"),
        }
    }

    pub fn random_pos(&self) -> (usize, usize) {
        let tile_x = get_u64() as usize % self.tiles_x;
        let tile_y = get_u64() as usize % self.tiles_y;
        let x = tile_x * self.tile_size + (self.tile_size - PLAYER_SIZE) / 2;
        let y = tile_y * self.tile_size + (self.tile_size - PLAYER_SIZE) / 2;
        (x, y)
    }

    pub fn is_pos_colliding(&self, x: isize, y: isize) -> bool {
        // Convert position to tile coordinates
        let tile_x = (x / self.tile_size as isize).max(0);
        let tile_y = (y / self.tile_size as isize).max(0);

        // TODO 4 tiles statt 9 (3x3) -> jeweils in richtung wo player näher dran ist
        let min_x = tile_x.saturating_sub(1);
        let max_x = (tile_x + 1).min(self.tiles_x as isize - 1);
        let min_y = tile_y.saturating_sub(1);
        let max_y = (tile_y + 1).min(self.tiles_y as isize - 1);

        // Collect all walls from surrounding tiles
        let mut walls = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let index = y * self.tiles_x as isize + x;
                if index < 0 || index as usize >= self.tiles.len() {
                    continue; // Skip out of bounds indices
                }
                let tile = &self.tiles[index as usize];

                if tile.up {
                    walls.push(self.wall_coords(x, y, 0))
                }
                if tile.right {
                    walls.push(self.wall_coords(x, y, 1));
                }
                if tile.down {
                    walls.push(self.wall_coords(x, y, 2));
                }
                if tile.left {
                    walls.push(self.wall_coords(x, y, 3));
                }
            }
        }

        // Player's collision lines
        let l = x;
        let r = x + PLAYER_SIZE as isize - 1;
        let t = y;
        let b = y + PLAYER_SIZE as isize - 1;

        // Check collision with each wall
        for &(wx1, wy1, wx2, wy2) in &walls {
            if line_intersects_line(l, t, r, t, wx1, wy1, wx2, wy2)
                || line_intersects_line(l, b, r, b, wx1, wy1, wx2, wy2)
                || line_intersects_line(l, t, l, b, wx1, wy1, wx2, wy2)
                || line_intersects_line(r, t, r, b, wx1, wy1, wx2, wy2)
            {
                return true;
            }
        }

        false
    }

    pub fn set_outer_walls(&mut self) {
        for x in 0..self.tiles_x {
            self.tiles[x].up = true; // Top row
            self.tiles[(self.tiles_y - 1) * self.tiles_x + x].down = true; // Bottom row
        }
        for y in 0..self.tiles_y {
            self.tiles[y * self.tiles_x].left = true; // Left column
            self.tiles[y * self.tiles_x + (self.tiles_x - 1)].right = true; // Right column
        }
    }

    pub fn set_tile(&mut self, x: usize, y: usize, walls: Direction) {
        if x < self.tiles_x && y < self.tiles_y {
            let index = y * self.tiles_x + x;
            self.tiles[index] = walls;
        } else {
            kprintln!("Coordinates out of bounds: ({}, {})", x, y);
        }
    }

    pub fn set_walls_bitmap(&mut self, bitmap: &[[bool; 4]]) {
        if bitmap.len() != self.tiles_x * self.tiles_y {
            panic!("Bitmap size does not match map dimensions");
        }
        for i in 0..(self.tiles_y * self.tiles_x) {
            // let x = i % self.tiles_x;
            // let y = i / self.tiles_x;
            let walls = Direction {
                up: bitmap[i][0],
                right: bitmap[i][1],
                down: bitmap[i][2],
                left: bitmap[i][3],
            };
            self.tiles[i] = walls;
        }
    }

    pub fn auto_set_walls(&mut self) {
        // clear existing walls
        for y in 0..self.tiles_y {
            for x in 0..self.tiles_x {
                let index = y * self.tiles_x + x;
                self.tiles[index] = Direction {
                    up: false,
                    right: false,
                    down: false,
                    left: false,
                };
            }
        }

        // Add random walls based on the density (only up and left walls to avoid double walls)
        for tile in &mut self.tiles {
            if random_float() < WALL_DENSITY.clamp(0.0, 1.0) {
                tile.up = true;
            }
            if random_float() < WALL_DENSITY.clamp(0.0, 1.0) {
                tile.left = true;
            }
        }

        // Set outer walls
        self.set_outer_walls();

        // remove closed areas
        self.ensure_accessible_areas();
    }

    fn ensure_accessible_areas(&mut self) {
        let mut visited = vec![false; self.tiles_x * self.tiles_y];
        let mut areas = Vec::new();

        // Find all distinct areas
        for y in 0..self.tiles_y {
            for x in 0..self.tiles_x {
                let index = y * self.tiles_x + x;
                if !visited[index] {
                    let mut area = Vec::new();
                    self.flood_fill(x, y, &mut visited, &mut area);
                    areas.push(area);
                }
            }
        }

        kprintln!("Found {} distinct areas", areas.len());

        // connect if more than one
        if areas.len() > 1 {
            // Sort areas by size (largest first)
            areas.sort_by(|a, b| b.len().cmp(&a.len()));

            let mut counter = 0;
            // Connect all smaller areas to the main (largest) area
            for i in 1..areas.len() {
                if let Some((x1, y1)) = areas[0].first() {
                    if let Some((x2, y2)) = areas[i].first() {
                        counter += 1;
                        self.connect_areas(*x1, *y1, *x2, *y2);
                    }
                }
            }
            kprintln!("Connected {} areas", counter);
        }
    }

    fn flood_fill(
        &self,
        x: usize,
        y: usize,
        visited: &mut Vec<bool>,
        area: &mut Vec<(usize, usize)>,
    ) {
        let index = y * self.tiles_x + x;
        if visited[index] {
            return;
        }

        visited[index] = true;
        area.push((x, y));

        // add neighbors if no wall in that direction
        let tile = &self.tiles[index];
        let tile_right = &self.tiles.get(index + 1).unwrap_or(&Direction {
            up: false,
            right: false,
            down: false,
            left: false,
        });
        let tile_down = &self.tiles.get(index + self.tiles_x).unwrap_or(&Direction {
            up: false,
            right: false,
            down: false,
            left: false,
        });
        if y > 0 && !tile.up {
            self.flood_fill(x, y - 1, visited, area);
        }
        // if x < self.tiles_x - 1 && !tile.right {
        if x < self.tiles_x - 1 && !tile.right && !tile_right.left {
            self.flood_fill(x + 1, y, visited, area);
        }
        // if y < self.tiles_y - 1 && !tile.down {
        if y < self.tiles_y - 1 && !tile.down && !tile_down.up {
            self.flood_fill(x, y + 1, visited, area);
        }
        if x > 0 && !tile.left {
            self.flood_fill(x - 1, y, visited, area);
        }
    }

    fn connect_areas(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        // simple pathfinding and remove all walls between two points
        let mut x = x1;
        let mut y = y1;

        while x != x2 || y != y2 {
            // Decide direction to move
            if x < x2 {
                // Remove right wall of current tile or left wall of next tile
                let index = y * self.tiles_x + x;
                self.tiles[index].right = false;
                if x < self.tiles_x - 1 {
                    let next_index = y * self.tiles_x + (x + 1);
                    self.tiles[next_index].left = false;
                }
                x += 1;
            } else if x > x2 {
                // Remove left wall of current tile or right wall of next tile
                let index = y * self.tiles_x + x;
                self.tiles[index].left = false;
                if x > 0 {
                    let next_index = y * self.tiles_x + (x - 1);
                    self.tiles[next_index].right = false;
                }
                x -= 1;
            }

            if y < y2 {
                // Remove down wall of current tile or up wall of next tile
                let index = y * self.tiles_x + x;
                self.tiles[index].down = false;
                if y < self.tiles_y - 1 {
                    let next_index = (y + 1) * self.tiles_x + x;
                    self.tiles[next_index].up = false;
                }
                y += 1;
            } else if y > y2 {
                // Remove up wall of current tile or down wall of next tile
                let index = y * self.tiles_x + x;
                self.tiles[index].up = false;
                if y > 0 {
                    let next_index = (y - 1) * self.tiles_x + x;
                    self.tiles[next_index].down = false;
                }
                y -= 1;
            }
        }
    }
}

fn random_float() -> f32 {
    (get_u64() as f32) / (u64::MAX as f32)
}

// Helper function to check if two line segments intersect
fn line_intersects_line(
    x1: isize,
    y1: isize,
    x2: isize,
    y2: isize,
    x3: isize,
    y3: isize,
    x4: isize,
    y4: isize,
) -> bool {
    let denom = (y4 - y3) * (x2 - x1) - (x4 - x3) * (y2 - y1);

    if denom == 0 {
        return false; // Lines are parallel
    }

    let ua = ((x4 - x3) * (y1 - y3) - (y4 - y3) * (x1 - x3)) as f64 / denom as f64;
    let ub = ((x2 - x1) * (y1 - y3) - (y2 - y1) * (x1 - x3)) as f64 / denom as f64;

    ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0
}

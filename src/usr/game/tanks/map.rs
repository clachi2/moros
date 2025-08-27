use crate::sys::rng::get_u64;
use crate::usr::game::tanks::state::{PLAYER_SIZE, Serializable, WALL_DENSITY};
use alloc::vec;
use alloc::vec::Vec;

/// Represents the wall directions for a tile in the map.
pub struct Direction {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
}

/// Clone implementation for Direction
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

/// Serializable implementation for Direction
impl Serializable for Direction {
    fn serialize(&self) -> Vec<u8> {
        let mut result = 0u8;
        if self.up {
            result |= 0b0001;
        }
        if self.right {
            result |= 0b0010;
        }
        if self.down {
            result |= 0b0100;
        }
        if self.left {
            result |= 0b1000;
        }
        vec![result]
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.is_empty() {
            return Direction {
                up: false,
                right: false,
                down: false,
                left: false,
            };
        }
        let byte = data[0];
        Direction {
            up: (byte & 0b0001) != 0,
            right: (byte & 0b0010) != 0,
            down: (byte & 0b0100) != 0,
            left: (byte & 0b1000) != 0,
        }
    }
}


/// Represents the game map with tiles and their wall configurations.
pub struct Map {
    pub size_x: usize,
    pub size_y: usize,
    pub tiles_x: usize,
    pub tiles_y: usize,
    pub tile_size: usize, // Size of each tile in pixels
    pub tiles: Vec<Direction>,
}


/// Clone implementation for Map
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
    /// Creates a new Map instance with the specified dimensions and tile configurations.
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

    /// Calculates the coordinates of a wall segment on a grid based on tile coordinates and facing direction.
    ///
    /// # Parameters
    /// - `x`, `y`:  Coordinates of the tile (zero-based indexing).
    /// - `facing`: Direction of the wall relative to the tile. Valid values are:
    ///   - `0`: Up, `1`: Right, `2`: Down, `3`: Left
    ///
    /// # Returns
    /// Returns a tuple `(x1, y1, x2, y2)` representing the starting and ending coordinates of the wall
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
                (x * ts, y * ts, (x + 1) * ts, y * ts) // up
            }
            1 => {
                ((x + 1) * ts - 1, y * ts, (x + 1) * ts - 1, (y + 1) * ts) // right
            }
            2 => {
                (x * ts, (y + 1) * ts - 1, (x + 1) * ts, (y + 1) * ts - 1) // down
            }
            3 => {
                (x * ts, y * ts, x * ts, (y + 1) * ts) // left
            }
            _ => panic!("Invalid facing direction"),
        }
    }

    /// Generates a random position within the bounds of a grid.
    ///
    /// # Returns
    /// A tuple `(usize, usize)` representing the x and y coordinates of the random position.
    /// The position is centered within a tile, accounting for the player's size, so that there are no collisions with walls.
    ///
    pub fn random_pos(&self) -> (usize, usize) {
        let tile_x = get_u64() as usize % self.tiles_x;
        let tile_y = get_u64() as usize % self.tiles_y;
        let x = tile_x * self.tile_size + (self.tile_size - PLAYER_SIZE) / 2;
        let y = tile_y * self.tile_size + (self.tile_size - PLAYER_SIZE) / 2;
        (x, y)
    }

    /// Sets walls around the outer edges of the map.
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

    /// Sets the wall configuration for a specific tile at (x, y).
    pub fn set_tile(&mut self, x: usize, y: usize, walls: Direction) {
        if x < self.tiles_x && y < self.tiles_y {
            let index = y * self.tiles_x + x;
            self.tiles[index] = walls;
        } else {
            kprintln!("Coordinates out of bounds: ({}, {})", x, y);
        }
    }

    /// Sets the wall configuration for each tile on the map using a bitmap representation.
    ///
    /// # Parameters
    /// - `bitmap`: A 2D slice of booleans (`&[[bool; 4]]`) where each inner array represents
    ///   the wall configuration of a single tile. Each inner array must correspond to the following
    ///   directions:
    ///     - Index `0`: `up`
    ///     - Index `1`: `right`
    ///     - Index `2`: `down`
    ///     - Index `3`: `left`
    ///   The number of elements in the bitmap must match the total number of tiles on the map, determined
    ///   by `self.tiles_x * self.tiles_y`.
    ///
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

    /// Automatically sets walls within the grid or map structure to ensure a connected layout.
    ///
    /// The method performs the following steps:
    /// 1. Clears all existing walls to start with a blank state.
    /// 2. Adds random walls within the grid based on a predefined wall density constant (`WALL_DENSITY`).
    /// 3. Sets the outer edges of the grid as walls to form boundaries.
    /// 4. Finds and identifies distinct areas (disjoint regions) within the grid.
    /// 5. Iteratively removes one wall from each distinct area until only one connected area remains.
    ///
    pub fn auto_set_walls(&mut self) {
        self.clear_walls();

        self.add_random_walls(WALL_DENSITY);

        self.set_outer_walls();

        let mut areas = self.find_distinct_areas();

        // Remove walls until only one area remains
        while areas.len() > 1 {
            self.remove_one_wall_per_area(&mut areas);
            areas = self.find_distinct_areas();
        }
    }

    /// Removes one wall per area.
    ///
    /// This function iterates over a collection of areas.
    /// For each tile in an area, it checks the neighboring tiles and removes one wall (up, right, down, or left)
    /// that separates the current area from another area. By doing so, it ensures there is a path connecting
    /// adjacent areas.
    ///
    /// # Parameters
    /// - `&mut self`: The mutable reference to the struct instance containing the `tiles` grid and its dimensions.
    /// - `mut areas: &mut Vec<Vec<(usize, usize)>>`: A mutable reference to a vector of areas, where each area is
    ///   represented as a vector of `(x, y)` coordinates corresponding to tiles in the grid.
    ///
    fn remove_one_wall_per_area(&mut self, mut areas: &mut Vec<Vec<(usize, usize)>>) {
        for area in areas.iter().take(areas.len() - 1) {
            // find one wall to remove (look in each tile of the area) -> break if found & removed
            for tile_coord in area {
                let x = tile_coord.0;
                let y = tile_coord.1;
                let index = y * self.tiles_x + x;
                let index_right = y * self.tiles_x + (x + 1); // right neighbor
                let index_down = (y + 1) * self.tiles_x + x; // down neighbor
                if y > 0
                    && self.tiles[index].up
                    && !area.contains(&(tile_coord.0, tile_coord.1 - 1))
                {
                    self.tiles[index].up = false;
                    break;
                }
                if x < self.tiles_x - 1
                    && (self.tiles[index].right || self.tiles[index_right].left)
                    && !area.contains(&(tile_coord.0 + 1, tile_coord.1))
                {
                    self.tiles[index].right = false;
                    self.tiles[index_right].left = false;
                    break;
                }
                if y < self.tiles_y - 1
                    && (self.tiles[index].down || self.tiles[index_down].up)
                    && !area.contains(&(tile_coord.0, tile_coord.1 + 1))
                {
                    self.tiles[index].down = false;
                    self.tiles[index_down].up = false;
                    break;
                }
                if x > 0
                    && self.tiles[index].left
                    && !area.contains(&(tile_coord.0 - 1, tile_coord.1))
                {
                    self.tiles[index].left = false;
                    break;
                }
            }
        }
    }


    /// Clears all the walls for each tile in the current collection.
    pub fn clear_walls(&mut self) {
        for tile in &mut self.tiles {
            tile.up = false;
            tile.right = false;
            tile.down = false;
            tile.left = false;
        }
    }

    /// Adds random walls to the tiles in the grid based on the specified density.
    ///
    /// This function assigns random walls (`up` and `left`) to each tile in the grid.
    /// To ensure no overlapping or double walls between tiles, only the `up` and `left`
    /// directions are considered for each tile. The probability of a wall being assigned
    /// is governed by the `density` parameter, which is clamped between `0.0` and `1.0`.
    ///
    /// # Parameters
    /// - `density` (`f32`): The density of walls to be added. A value of `0.0` means no
    ///   walls will be added, while `1.0` means every tile will receive walls in the `up`
    ///   and `left` directions.
    ///
    pub fn add_random_walls(&mut self, density: f32) {
        // Add random walls based on the density (only up and left walls to avoid double walls)
        for tile in &mut self.tiles {
            tile.up = random_float() < density.clamp(0.0, 1.0);
            tile.left = random_float() < density.clamp(0.0, 1.0);
        }
    }


    /// Finds and returns distinct areas within a grid.
    ///
    /// This method iterates over all tiles in a grid represented by `self.tiles_x` (width) and `self.tiles_y` (height).
    /// It identifies connected regions (areas) using a flood-fill algorithm, marking visited tiles as it proceeds to
    /// ensure areas are distinct and non-overlapping.
    ///
    /// Each area is represented as a `Vec<(usize, usize)>`, where each tuple corresponds to the `(x, y)` coordinates of
    /// the tiles belonging to that area. The method returns a vector of these areas.
    ///
    pub fn find_distinct_areas(&self) -> Vec<Vec<(usize, usize)>> {
        let mut visited = vec![false; self.tiles_x * self.tiles_y];
        let mut areas = Vec::new();

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

        areas
    }

    /// Performs a flood-fill algorithm to identify a connected area in a grid starting from the provided coordinates.
    ///
    /// This method recursively explores neighboring tiles in the grid to determine all tiles that are part of
    /// a contiguous region without walls blocking the way.
    ///
    /// # Parameters
    /// - `x`: The x-coordinate of the starting tile in the grid.
    /// - `y`: The y-coordinate of the starting tile in the grid.
    /// - `visited`: A mutable vector of booleans representing whether a specific tile in the grid
    ///   has already been visited. Its length should be `self.tiles_x * self.tiles_y`.
    /// - `area`: A mutable vector containing tuples of `(usize, usize)`
    ///   representing coordinates of tiles in the connected area. The function will populate this vector
    ///   with all tiles that belong to the same contiguous area as the starting tile.
    ///
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
        if x < self.tiles_x - 1 && !tile.right && !tile_right.left {
            self.flood_fill(x + 1, y, visited, area);
        }
        if y < self.tiles_y - 1 && !tile.down && !tile_down.up {
            self.flood_fill(x, y + 1, visited, area);
        }
        if x > 0 && !tile.left {
            self.flood_fill(x - 1, y, visited, area);
        }
    }
}

/// Generates a random floating-point number between 0.0 and 1.0.
fn random_float() -> f32 {
    (get_u64() as f32) / (u64::MAX as f32)
}

/// Serializable implementation for Map
impl Serializable for Map {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize map dimensions (4 bytes each)
        result.extend_from_slice(&(self.size_x as u32).to_le_bytes());
        result.extend_from_slice(&(self.size_y as u32).to_le_bytes());
        result.extend_from_slice(&(self.tiles_x as u32).to_le_bytes());
        result.extend_from_slice(&(self.tiles_y as u32).to_le_bytes());
        result.extend_from_slice(&(self.tile_size as u32).to_le_bytes());

        // Serialize tiles
        for tile in &self.tiles {
            result.extend(tile.serialize());
        }

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 20 {
            panic!("Invalid map data: too short");
        }

        let size_x = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let size_y = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let tiles_x = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let tiles_y = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let tile_size = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;

        let expected_tiles = tiles_x * tiles_y;
        if data.len() < 20 + expected_tiles {
            panic!("Invalid map data: not enough tile data");
        }

        let mut tiles = Vec::new();
        for i in 0..expected_tiles {
            let tile_data = &data[20 + i..20 + i + 1];
            tiles.push(Direction::deserialize(tile_data));
        }

        Map {
            size_x,
            size_y,
            tiles_x,
            tiles_y,
            tile_size,
            tiles,
        }
    }
}
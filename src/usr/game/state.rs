use crate::kprintln;
use crate::sys::rng::get_u64;
use alloc::vec;
use alloc::vec::Vec;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::Player;

pub static PLAYER_SPEED: f64 = 40.0; // Speed of player movement per tick
pub static PLAYER_SIZE: usize = 8; // Size of the player in pixels
pub static BULLET_SPEED: f64 = 40.0; // Speed of bullet movement per tick
pub static BULLET_TRAVEL_DIST: f64 = 200.0; // Maximum travel distance for bullets
pub static SHOOTING_RATE_PER_SECOND: f64 = 1000.0; // Maximum travel distance for bullets
pub static GUI_WIDTH: usize = 80;
pub static TICK_RATE: f64 = 64.0; // Number of ticks per second
pub static WALL_DENSITY: f32 = 0.5; // Density of walls in the map

pub(crate) trait Serializable {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Self;
} // TODO für alle structs implementieren

pub(crate) struct GameState {
    pub(crate) map: Map,
    pub(crate) players: Vec<Player>,
    pub(crate) bullets: Vec<Bullet>,
    pub(crate) current_player_index: usize,
    pub(crate) last_tick: f64, // Timestamp
}

pub(crate) struct UserInput {
    // keyboard input
    pub(crate) up: bool,
    pub(crate) right: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    // mouse input
    pub(crate) shooting: bool,
    pub(crate) map_mouse_x: isize,
    pub(crate) map_mouse_y: isize,
}

impl Clone for UserInput {
    fn clone(&self) -> Self {
        UserInput {
            up: self.up,
            right: self.right,
            down: self.down,
            left: self.left,
            shooting: self.shooting,
            map_mouse_x: self.map_mouse_x,
            map_mouse_y: self.map_mouse_y,
        }
    }
}

impl Serializable for UserInput {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Pack boolean inputs into a single byte
        let mut input_flags = 0u8;
        if self.up {
            input_flags |= 0b00001;
        }
        if self.right {
            input_flags |= 0b00010;
        }
        if self.down {
            input_flags |= 0b00100;
        }
        if self.left {
            input_flags |= 0b01000;
        }
        if self.shooting {
            input_flags |= 0b10000;
        }
        result.push(input_flags);

        // Serialize mouse coordinates (4 bytes each)
        result.extend_from_slice(&(self.map_mouse_x as i32).to_le_bytes());
        result.extend_from_slice(&(self.map_mouse_y as i32).to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 9 {
            return UserInput {
                up: false,
                right: false,
                down: false,
                left: false,
                shooting: false,
                map_mouse_x: 0,
                map_mouse_y: 0,
            };
        }

        let input_flags = data[0];
        let map_mouse_x = i32::from_le_bytes([data[1], data[2], data[3], data[4]]) as isize;
        let map_mouse_y = i32::from_le_bytes([data[5], data[6], data[7], data[8]]) as isize;

        UserInput {
            up: (input_flags & 0b00001) != 0,
            right: (input_flags & 0b00010) != 0,
            down: (input_flags & 0b00100) != 0,
            left: (input_flags & 0b01000) != 0,
            shooting: (input_flags & 0b10000) != 0,
            map_mouse_x,
            map_mouse_y,
        }
    }
}

pub(crate) struct Player {
    pub(crate) id: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) alive: bool,
    pub(crate) time_of_death: f64, // Timestamp of death (get using time::epoch_time())
    pub(crate) user_input: UserInput,
    pub(crate) pointing_to: (usize, usize),
    pub(crate) color: u8,
    pub(crate) points: usize,
    pub(crate) ammo: usize,
    pub(crate) last_shot: f64, // Timestamp of the last shot (get using time::epoch_time())
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Player {
            id: self.id,
            x: self.x,
            y: self.y,
            alive: self.alive,
            time_of_death: self.time_of_death,
            user_input: self.user_input.clone(),
            pointing_to: self.pointing_to,
            color: self.color,
            points: self.points,
            ammo: self.ammo,
            last_shot: self.last_shot,
        }
    }
}

impl Player {
    pub fn next_wanted_position(&self, tick_delta: f64) -> (f64, f64) {
        let mut new_x = self.x;
        let mut new_y = self.y;
        if self.user_input.up {
            new_y -= tick_delta * PLAYER_SPEED; // Adjust speed as needed
        }
        if self.user_input.down {
            new_y += tick_delta * PLAYER_SPEED; // Adjust speed as needed
        }
        if self.user_input.left {
            new_x -= tick_delta * PLAYER_SPEED; // Adjust speed as needed
        }
        if self.user_input.right {
            new_x += tick_delta * PLAYER_SPEED; // Adjust speed as needed
        }
        (new_x, new_y)
    }
}

impl Serializable for Player {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize player ID (4 bytes)
        result.extend_from_slice(&(self.id as u32).to_le_bytes());

        // Serialize position (8 bytes each for x and y as f64)
        result.extend_from_slice(&self.x.to_le_bytes());
        result.extend_from_slice(&self.y.to_le_bytes());

        // Serialize alive status (1 byte)
        result.push(if self.alive { 1 } else { 0 });

        // Serialize time_of_death (8 bytes)
        result.extend_from_slice(&self.time_of_death.to_le_bytes());

        // Serialize user input
        result.extend(self.user_input.serialize());

        // Serialize pointing_to (4 bytes each)
        result.extend_from_slice(&(self.pointing_to.0 as u32).to_le_bytes());
        result.extend_from_slice(&(self.pointing_to.1 as u32).to_le_bytes());

        // Serialize color (1 byte)
        result.push(self.color);

        // Serialize points (4 bytes)
        result.extend_from_slice(&(self.points as u32).to_le_bytes());

        // Serialize ammo (4 bytes)
        result.extend_from_slice(&(self.ammo as u32).to_le_bytes());

        // Serialize last_shot (8 bytes)
        result.extend_from_slice(&self.last_shot.to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 58 {
            // Minimum size for all fields
            return Player {
                id: 0,
                x: 0.0,
                y: 0.0,
                alive: true,
                time_of_death: 0.0,
                user_input: UserInput {
                    up: false,
                    right: false,
                    down: false,
                    left: false,
                    shooting: false,
                    map_mouse_x: 0,
                    map_mouse_y: 0,
                },
                pointing_to: (0, 0),
                color: 0,
                points: 0,
                ammo: 0,
                last_shot: 0.0,
            };
        }

        let id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let x = f64::from_le_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);
        let y = f64::from_le_bytes([
            data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
        ]);
        let alive = data[20] != 0;
        let time_of_death = f64::from_le_bytes([
            data[21], data[22], data[23], data[24], data[25], data[26], data[27], data[28],
        ]);

        // Deserialize user input (9 bytes)
        let user_input = UserInput::deserialize(&data[29..38]);

        let pointing_to_x = u32::from_le_bytes([data[38], data[39], data[40], data[41]]) as usize;
        let pointing_to_y = u32::from_le_bytes([data[42], data[43], data[44], data[45]]) as usize;
        let color = data[46];
        let points = u32::from_le_bytes([data[47], data[48], data[49], data[50]]) as usize;
        let ammo = u32::from_le_bytes([data[51], data[52], data[53], data[54]]) as usize;
        let last_shot = f64::from_le_bytes([
            data[55], data[56], data[57], data[58], data[59], data[60], data[61], data[62],
        ]);

        Player {
            id,
            x,
            y,
            alive,
            time_of_death,
            user_input,
            pointing_to: (pointing_to_x, pointing_to_y),
            color,
            points,
            ammo,
            last_shot,
        }
    }
}

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

pub(crate) struct Bullet {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) direction: Direction,
    pub(crate) started_at_x: f64,
    pub(crate) started_at_y: f64,
    pub(crate) travel_distance: usize,
    // TODO direkt pfad komplett ausrechnen und speichern?
    // -> dann immer delta auf dem phad weiter gehen (float)
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
                (x * ts, y * ts, (x + 1) * ts - 1, y * ts) // up
            }
            1 => {
                ((x + 1) * ts - 1, y * ts, (x + 1) * ts - 1, (y + 1) * ts - 1) // right
            }
            2 => {
                (x * ts, (y + 1) * ts - 1, (x + 1) * ts - 1, (y + 1) * ts - 1) // down
            }
            3 => {
                (x * ts, y * ts, x * ts, (y + 1) * ts - 1) // left
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
            let x = i % self.tiles_x;
            let y = i / self.tiles_x;
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

        // Add random walls based on the density
        for y in 0..self.tiles_y {
            for x in 0..self.tiles_x {
                if self.random_float() < WALL_DENSITY.clamp(0.0, 1.0) {
                    self.try_add_wall(x, y);
                }
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

        // connect if more than one
        if areas.len() > 1 {
            // Sort areas by size (largest first)
            areas.sort_by(|a, b| b.len().cmp(&a.len()));

            // Connect all smaller areas to the main (largest) area
            for i in 1..areas.len() {
                if let Some((x1, y1)) = areas[0].first() {
                    if let Some((x2, y2)) = areas[i].first() {
                        self.connect_areas(*x1, *y1, *x2, *y2);
                    }
                }
            }
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
        if y > 0 && !tile.up {
            self.flood_fill(x, y - 1, visited, area);
        }
        if x < self.tiles_x - 1 && !tile.right {
            self.flood_fill(x + 1, y, visited, area);
        }
        if y < self.tiles_y - 1 && !tile.down {
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

    fn try_add_wall(&mut self, x: usize, y: usize) {
        let index = y * self.tiles_x + x;
        let dir = (get_u64() % 4) as usize;

        match dir {
            0 => {
                // up
                if y > 1 {
                    self.tiles[index].up = true;
                    self.tiles[(y - 1) * self.tiles_x + x].down = true;
                }
            }
            1 => {
                // right
                if x < self.tiles_x - 2 {
                    self.tiles[index].right = true;
                    self.tiles[y * self.tiles_x + (x + 1)].left = true;
                }
            }
            2 => {
                // down
                if y < self.tiles_y - 2 {
                    self.tiles[index].down = true;
                    self.tiles[(y + 1) * self.tiles_x + x].up = true;
                }
            }
            3 => {
                // left
                if x > 1 {
                    self.tiles[index].left = true;
                    self.tiles[y * self.tiles_x + (x - 1)].right = true;
                }
            }
            _ => {}
        }
    }

    fn random_float(&self) -> f32 {
        (get_u64() as f32) / (u64::MAX as f32)
    }
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
    // Implementation of line segment intersection test
    let denom = (y4 - y3) * (x2 - x1) - (x4 - x3) * (y2 - y1);

    if denom == 0 {
        return false; // Lines are parallel
    }

    let ua = ((x4 - x3) * (y1 - y3) - (y4 - y3) * (x1 - x3)) as f64 / denom as f64;
    let ub = ((x2 - x1) * (y1 - y3) - (y2 - y1) * (x1 - x3)) as f64 / denom as f64;

    ua >= 0.0 && ua <= 1.0 && ub >= 0.0 && ub <= 1.0
}

impl Serializable for GameState {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize map
        let map_data = self.map.serialize();
        result.extend_from_slice(&(map_data.len() as u32).to_le_bytes());
        result.extend(map_data);

        // Serialize players count and players
        result.extend_from_slice(&(self.players.len() as u32).to_le_bytes());
        for player in &self.players {
            let player_data = player.serialize();
            result.extend_from_slice(&(player_data.len() as u32).to_le_bytes());
            result.extend(player_data);
        }

        // Serialize bullets count (placeholder for now)
        result.extend_from_slice(&(0u32).to_le_bytes()); // No bullets serialized yet

        // Serialize current_player_index (4 bytes)
        result.extend_from_slice(&(self.current_player_index as u32).to_le_bytes());

        // Serialize last_tick (8 bytes)
        result.extend_from_slice(&self.last_tick.to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 20 {
            return GameState::new(Map::new(320, 200, 12, 10, 20));
        }

        let mut offset = 0;

        // Deserialize map
        let map_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        if data.len() < offset + map_len {
            return GameState::new(Map::new(320, 200, 12, 10, 20));
        }

        let map = Map::deserialize(&data[offset..offset + map_len]);
        offset += map_len;

        // Deserialize players
        if data.len() < offset + 4 {
            return GameState::new(map);
        }

        let players_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let mut players = Vec::new();
        for _ in 0..players_count {
            if data.len() < offset + 4 {
                break;
            }

            let player_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if data.len() < offset + player_len {
                break;
            }

            let player = Player::deserialize(&data[offset..offset + player_len]);
            players.push(player);
            offset += player_len;
        }

        // Skip bullets count (not implemented yet)
        if data.len() >= offset + 4 {
            offset += 4;
        }

        // Deserialize current_player_index
        let current_player_index = if data.len() >= offset + 4 {
            let index = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            index
        } else {
            0
        };

        // Deserialize last_tick
        let last_tick = if data.len() >= offset + 8 {
            f64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ])
        } else {
            0.0
        };

        GameState {
            map,
            players,
            bullets: Vec::new(), // Not implemented yet
            current_player_index,
            last_tick,
        }
    }
}

impl GameState {
    pub fn new(map: Map) -> Self {
        GameState {
            map,
            players: Vec::new(),
            bullets: Vec::new(),
            current_player_index: 0,
            last_tick: 0.0, // Initialize with a default value
        }
    }
}


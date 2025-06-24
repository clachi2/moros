use alloc::vec;
use alloc::vec::Vec;

pub static PLAYER_SPEED: f64 = 20.0; // Speed of player movement per tick
pub static PLAYER_SIZE: usize = 8; // Size of the player in pixels
pub static BULLET_SPEED: f64 = 20.0; // Speed of bullet movement per tick
pub static BULLET_SIZE: usize = 4; // Size of the bullet in pixels
pub static GUI_WIDTH: usize = 80;
pub static TICK_RATE: f64 = 32.0; // Number of ticks per second

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

pub(crate) struct Map {
    pub(crate) size_x: usize,
    pub(crate) size_y: usize,
    pub(crate) tiles_x: usize,
    pub(crate) tiles_y: usize,
    pub(crate) tiles: Vec<Direction>,
}

impl Clone for Map {
    fn clone(&self) -> Self {
        Map {
            size_x: self.size_x,
            size_y: self.size_y,
            tiles_x: self.tiles_x,
            tiles_y: self.tiles_y,
            tiles: self.tiles.clone(),
        }
    }
}

pub(crate) struct UserInput {
    // keyboard input
    pub(crate) up: bool,
    pub(crate) right: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    // mouse input
    pub(crate) shooting: bool,
    pub(crate) map_mouse_x: usize,
    pub(crate) map_mouse_y: usize,
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

pub(crate) struct Player {
    pub(crate)id: usize,
    pub(crate)x: f64, // TODO vll als float, damit ticks funktionieren?
    pub(crate)y: f64,
    pub(crate)alive: bool,
    pub(crate)time_of_death: f64, // Timestamp of death (get using time::epoch_time())
    pub(crate)user_input: UserInput,
    pub(crate)pointing_to: (usize, usize),
    pub(crate)color: u8,
    pub(crate)points: usize,
    pub(crate)ammo: usize,
    pub(crate)last_shot: f64, // Timestamp of the last shot (get using time::epoch_time())
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


impl Player{
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

pub(crate)struct Bullet {
    pub(crate)x: f64, // TODO vll als float, damit ticks funktionieren?
    pub(crate)y: f64,
    pub(crate)direction: Direction,
    pub(crate)started_at_x: f64,
    pub(crate)started_at_y: f64,
    pub(crate)travel_distance: usize,
}

impl Map {
    pub fn new(size_x: usize, size_y: usize, tiles_x: usize, tiles_y: usize) -> Self {
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
            tiles: tile_walls,
        }
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

    pub fn auto_set_inner_walls(&mut self) {
        // TODO: Implement random wall generation logic
    }

    pub fn set_test_map(&mut self) {
        let walls_bitmap = [
            // row 1
            [true, false, false, true],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, true, false],
            [true, false, true, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, true, false, false],
            // row 2
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
            [true, false, false, true],
            [true, false, false, false],
            [false, false, false, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
            // row 3
            [false, false, false, true],
            [false, false, true, false],
            [false, false, false, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, true, false, false],
            [true, false, false, true],
            [true, false, false, false],
            [false, false, false, false],
            [false, false, true, false],
            [false, true, false, false],
            // row 4
            [false, false, false, true],
            [true, true, false, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, true, false, false],
            [true, false, false, true],
            [false, true, false, false],
            // row 5
            [false, false, false, true],
            [false, true, false, false],
            [false, false, false, true],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, true, false, false],
            // row 6
            [false, false, false, true],
            [false, true, false, false],
            [false, false, false, true],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, true, false, false],
            // row 7
            [false, false, false, true],
            [false, true, true, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
            [false, false, true, true],
            [false, true, false, false],
            // row 8
            [false, false, false, true],
            [true, false, false, false],
            [false, false, false, false],
            [false, false, true, false],
            [false, true, true, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, true, false, false],
            [false, false, false, true],
            [false, false, false, false],
            [true, false, false, false],
            [false, true, false, false],
            // row 9
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [true, false, false, false],
            [true, false, false, false],
            [false, false, false, false],
            [false, false, true, false],
            [false, true, true, false],
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, true, false, false],
            // row 10
            [false, false, true, true],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [true, false, true, false],
            [true, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, false, true, false],
            [false, true, true, false],
        ];
        self.set_walls_bitmap(&walls_bitmap);
    }
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            map: Map::new(320, 200, 12, 10), // Example dimensions, adjust as needed
            players: Vec::new(),
            bullets: Vec::new(),
            current_player_index: 0,
            last_tick: 0.0, // Initialize with a default value
        }
    }
}

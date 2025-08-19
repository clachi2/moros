use crate::usr::game::tanks::map::Map;
use crate::usr::game::tanks::state::Serializable;
use crate::usr::game::tanks::state::{PLAYER_SIZE, PLAYER_SPEED};
use crate::usr::game::tanks::userinput::UserInput;
use alloc::vec::Vec;
use num_traits::Float;

/// Represents a player in the game with attributes such as position, state, user input, and game statistics.
pub struct Player {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub alive: bool,
    pub time_of_death: f64, // Timestamp of death (get using time::epoch_time())
    pub user_input: UserInput,
    pub pointing_to: (usize, usize),
    pub points: usize,
    pub ammo: usize,
    pub last_shot: f64, // Timestamp of the last shot (get using time::epoch_time())
    pub last_reload_ammo: f64, // Timestamp of the last ammo reload
}

impl Clone for Player {
    /// Creates a new `Player` instance that is a copy of the current player.
    fn clone(&self) -> Self {
        Player {
            id: self.id,
            x: self.x,
            y: self.y,
            alive: self.alive,
            time_of_death: self.time_of_death,
            user_input: self.user_input.clone(),
            pointing_to: self.pointing_to,
            points: self.points,
            ammo: self.ammo,
            last_shot: self.last_shot,
            last_reload_ammo: 0.0, // Initialize with a default value
        }
    }
}

impl Player {
    /// Updates the player's position on a map based on user input, elapsed time, and collision detection.
    ///
    /// This function is responsible for incrementally moving the player's position while avoiding collisions
    /// with obstacles in the environment. Movement is determined by the player's input, which specifies
    /// the directions for movement (up, down, left, right).
    ///
    /// # Parameters
    ///
    /// * `tick_delta`: A floating-point value representing the time elapsed since the last update.
    ///   This is used to calculate the movement distance based on the player's speed.
    /// * `map`: A reference to the `Map` object, which provides information about the game's environment
    ///
    pub fn update_position(&mut self, tick_delta: f64, map: &Map) {
        // Calculate the movement direction based on user input
        let dx = if self.user_input.left {
            -1.0
        } else if self.user_input.right {
            1.0
        } else {
            0.0
        };
        let dy = if self.user_input.up {
            -1.0
        } else if self.user_input.down {
            1.0
        } else {
            0.0
        };
        // Calculate the total movement length based on the player's speed and the elapsed time
        let move_length = tick_delta * PLAYER_SPEED;
        let mut moved_length = 0.0;
        loop {
            let next_x = self.x + dx;
            let next_y = self.y + dy;
            let mut moved_x = false;
            let mut moved_y = false;
            // Check for collisions in the next position
            if !self.is_pos_colliding(next_x as isize, self.y as isize, map) {
                moved_x = true;
            }
            if !self.is_pos_colliding(self.x as isize, next_y as isize, map) {
                moved_y = true;
            }
            // diagonal movement
            if moved_x && moved_y {
                if moved_length + 2.0.sqrt() <= move_length {
                    moved_length += 2.0.sqrt();
                    self.x = next_x; // move horizontally if no collision
                    self.y = next_y; // move vertically if no collision
                } else {
                    break;
                }
            }
            // horizontal movement
            else if moved_x {
                if moved_length + 1.0 <= move_length {
                    moved_length += 1.0; // move in one direction
                    self.x = next_x; // move horizontally if no collision
                } else {
                    break;
                }
            }
            // vertical movement
            else if moved_y {
                if moved_length + 1.0 <= move_length {
                    moved_length += 1.0; // move in one direction
                    self.y = next_y; // move vertically if no collision
                } else {
                    break;
                }
            } else {
                break; // stop moving if both directions are blocked
            }
        }

        // add rest after comma (if last full movement step was colliding, but there is still some distance left to move)
        let rest = (move_length - moved_length).fract();
        if rest > 0.0 {
            let length = (dx * dx + dy * dy).sqrt();
            if length > 0.0 {
                let scale = rest / length;
                // Try to move in the direction of the remaining distance
                let next_x = self.x + dx * scale;
                let next_y = self.y + dy * scale;
                if !self.is_pos_colliding(next_x as isize, self.y as isize, map) {
                    self.x = next_x; // move horizontally if no collision
                }
                if !self.is_pos_colliding(self.x as isize, next_y as isize, map) {
                    self.y = next_y; // move vertically if no collision
                }
            }
        }
    }

    /// Determines if a given position is colliding with any walls in the map.
    ///
    /// # Parameters
    /// - `x`: The x-coordinate of the position to check, in world units.
    /// - `y`: The y-coordinate of the position to check, in world units.
    /// - `map`: A reference to the `Map` object, which contains information about the tile layout and wall positions.
    ///
    /// # Returns
    /// - `true` if the position collides with any walls; otherwise, `false`.
    fn is_pos_colliding(&self, x: isize, y: isize, map: &Map) -> bool {
        // Convert position to tile coordinates
        let tile_x = (x / map.tile_size as isize).max(0);
        let tile_y = (y / map.tile_size as isize).max(0);

        // Collects 3x3 surrounding tile-indices within the map bounds
        let min_x = tile_x.saturating_sub(1);
        let max_x = (tile_x + 1).min(map.tiles_x as isize - 1);
        let min_y = tile_y.saturating_sub(1);
        let max_y = (tile_y + 1).min(map.tiles_y as isize - 1);

        // Collect all walls from surrounding tiles
        let mut walls = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let index = y * map.tiles_x as isize + x;
                if index < 0 || index as usize >= map.tiles.len() {
                    continue; // Skip out of bounds indices
                }
                let tile = &map.tiles[index as usize];

                if tile.up {
                    walls.push(map.wall_coords(x, y, 0))
                }
                if tile.right {
                    walls.push(map.wall_coords(x, y, 1));
                }
                if tile.down {
                    walls.push(map.wall_coords(x, y, 2));
                }
                if tile.left {
                    walls.push(map.wall_coords(x, y, 3));
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
}

impl Serializable for Player {
    /// Serializes the `Player` instance into a byte vector.
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

        // Serialize points (4 bytes)
        result.extend_from_slice(&(self.points as u32).to_le_bytes());

        // Serialize ammo (4 bytes)
        result.extend_from_slice(&(self.ammo as u32).to_le_bytes());

        // Serialize last_shot (8 bytes)
        result.extend_from_slice(&self.last_shot.to_le_bytes());

        // Serialize last_reload_ammo
        result.extend_from_slice(&self.last_reload_ammo.to_le_bytes());

        result
    }

    /// Deserializes a byte slice into a `Player` instance.
    ///
    /// If the data is too short, it returns a default `Player` with all fields set to false and values to 0.
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
                points: 0,
                ammo: 0,
                last_shot: 0.0,
                last_reload_ammo: 0.0,
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

        let points = u32::from_le_bytes([data[46], data[47], data[48], data[49]]) as usize;
        let ammo = u32::from_le_bytes([data[50], data[51], data[52], data[53]]) as usize;
        let last_shot = f64::from_le_bytes([
            data[54], data[55], data[56], data[57], data[58], data[59], data[60], data[61],
        ]);
        let last_reload_ammo = f64::from_le_bytes([
            data[62], data[63], data[64], data[65], data[66], data[67], data[68], data[69],
        ]);

        Player {
            id,
            x,
            y,
            alive,
            time_of_death,
            user_input,
            pointing_to: (pointing_to_x, pointing_to_y),
            points,
            ammo,
            last_shot,
            last_reload_ammo,
        }
    }
}

/// Determines if two line segments intersect.
///
/// This function checks whether the line segment from `(x1, y1)` to `(x2, y2)`
/// intersects with the line segment from `(x3, y3)` to `(x4, y4)`.
///
/// # Returns
///
/// Returns `true` if the two line segments intersect, and `false` otherwise.
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

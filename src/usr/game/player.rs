use crate::usr::game::game::UserInput;
use crate::usr::game::map::Map;
use crate::usr::game::state::{PLAYER_SIZE, PLAYER_SPEED};
use alloc::vec::Vec;
use num_traits::Float;

pub(crate) struct Player {
    pub(crate) id: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) alive: bool,
    pub(crate) time_of_death: f64, // Timestamp of death (get using time::epoch_time())
    pub(crate) user_input: UserInput,
    pub(crate) pointing_to: (usize, usize),
    pub(crate) points: usize,
    pub(crate) ammo: usize,
    pub(crate) last_shot: f64, // Timestamp of the last shot (get using time::epoch_time())
    pub(crate) last_reload_ammo: f64, // Timestamp of the last ammo reload
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
            points: self.points,
            ammo: self.ammo,
            last_shot: self.last_shot,
            last_reload_ammo: 0.0, // Initialize with a default value
        }
    }
}

impl Player {
    pub fn update_position(&mut self, tick_delta: f64, map: &Map) {
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
        let move_length = tick_delta * PLAYER_SPEED;
        let mut moved_length = 0.0;
        loop {
            let next_x = self.x + dx;
            let next_y = self.y + dy;
            let mut moved_x = false;
            let mut moved_y = false;
            if !self.is_pos_colliding(next_x as isize, self.y as isize, map) {
                moved_x = true;
            }
            if !self.is_pos_colliding(self.x as isize, next_y as isize, map) {
                moved_y = true;
            }
            if moved_x && moved_y {
                if moved_length + 2.0.sqrt() <= move_length {
                    moved_length += 2.0.sqrt();
                    self.x = next_x; // move horizontally if no collision
                    self.y = next_y; // move vertically if no collision
                } else {
                    break;
                }
            } else if moved_x {
                if moved_length + 1.0 <= move_length {
                    moved_length += 1.0; // move in one direction
                    self.x = next_x; // move horizontally if no collision
                } else {
                    break;
                }
            } else if moved_y {
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

        // add rest after comma
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

    fn is_pos_colliding(&self, x: isize, y: isize, map: &Map) -> bool {
        // Convert position to tile coordinates
        let tile_x = (x / map.tile_size as isize).max(0);
        let tile_y = (y / map.tile_size as isize).max(0);

        // TODO 4 tiles statt 9 (3x3) -> jeweils in richtung wo player näher dran ist
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

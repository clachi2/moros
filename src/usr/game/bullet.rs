use crate::kprintln;
use crate::usr::game::map::Map;
use crate::usr::game::state::{BULLET_SPEED, BULLET_TRAVEL_DIST, Serializable};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use num_traits::Float;

// #[derive(Clone)]
// pub(crate) struct PathPoint {
//     pub(crate) x: f64,
//     pub(crate) y: f64,
// }

pub(crate) struct Bullet {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) pointing_towards_x: f64,
    pub(crate) pointing_towards_y: f64,
    pub(crate) already_traveled: f64,
    // pub(crate) path_queue: VecDeque<PathPoint>, // Pre-calculated path points
    pub(crate) path_queue: VecDeque<(f64, f64)>, // Pre-calculated path points
    // pub(crate) current_segment_start: PathPoint, // Start of current segment
    pub(crate) current_segment_start: (f64, f64), // Start of current segment
                                                  // TODO direkt pfad komplett ausrechnen und speichern?
                                                  // pro pixel ob x oder/und y colliding -> wenn beide dann 180 grad, wenn nur eins dann spiegeln an wand
                                                  // immer goto-points bis zu nächsten wand (oder ende des pfades)
                                                  // -> dann immer delta auf dem phad weiter gehen (float)
}

impl Bullet {
    pub fn new(start_x: f64, start_y: f64, target_x: f64, target_y: f64, map: &Map) -> Self {
        let mut bullet = Bullet {
            x: start_x,
            y: start_y,
            pointing_towards_x: target_x,
            pointing_towards_y: target_y,
            already_traveled: 0.0,
            path_queue: VecDeque::new(),
            // current_segment_start: PathPoint { x: start_x, y: start_y },
            current_segment_start: (start_x, start_y),
        };

        // Calculate the entire path with reflections
        bullet.calculate_path(map);
        bullet
    }

    fn calculate_path(&mut self, map: &Map) {
        let mut current_x = self.x;
        let mut current_y = self.y;
        let mut direction_x = self.pointing_towards_x - self.x;
        let mut direction_y = self.pointing_towards_y - self.y;

        // Normalize direction
        let length = (direction_x * direction_x + direction_y * direction_y).sqrt();
        if length > 0.0 {
            direction_x /= length;
            direction_y /= length;
        }

        let mut already_traveled = 0.0;

        while already_traveled < BULLET_TRAVEL_DIST {
            if let Some((col_x, col_y, norm_x, norm_y, dist)) =
                self.find_next_collision(current_x, current_y, direction_x, direction_y, map)
            {
                if already_traveled + dist >= BULLET_TRAVEL_DIST {
                    // not reaching next collision
                    let remaining_dist = BULLET_TRAVEL_DIST - already_traveled;
                    self.path_queue.push_back((
                        current_x + direction_x * remaining_dist,
                        current_y + direction_y * remaining_dist,
                    ));
                    break;
                }

                self.path_queue.push_back((col_x, col_y));

                (current_x, current_y) = self.clamp_to_map_bounds_and_round(col_x, col_y, map);

                already_traveled += dist;

                // calc reflection
                let dot = direction_x * norm_x + direction_y * norm_y;
                direction_x -= 2.0 * dot * norm_x;
                direction_y -= 2.0 * dot * norm_y;
            } else {
                // no collision found
                let remaining_dist = BULLET_TRAVEL_DIST - already_traveled;
                let final_x = current_x + direction_x * remaining_dist;
                let final_y = current_y + direction_y * remaining_dist;
                self.path_queue.push_back((final_x, final_y));
                break;
            }
        }

        // dequeue first point
        if let Some(first_point) = self.path_queue.front() {
            self.pointing_towards_x = first_point.0;
            self.pointing_towards_y = first_point.1;
        }
    }

    fn clamp_to_map_bounds_and_round(&self, x: f64, y: f64, map: &Map) -> (f64, f64) {
        let max_x = (map.tiles_x * map.tile_size) as f64;
        let max_y = (map.tiles_y * map.tile_size) as f64;

        let clamped_x = x.max(0.0).min(max_x);
        let clamped_y = y.max(0.0).min(max_y);

        let rounded_x = (clamped_x * 1e8).round() / 1e8;
        let rounded_y = (clamped_y * 1e8).round() / 1e8;

        (rounded_x, rounded_y)
    }

    fn find_next_collision(
        &self,
        start_x: f64,
        start_y: f64,
        dir_x: f64,
        dir_y: f64,
        map: &Map,
    ) -> Option<(f64, f64, f64, f64, f64)> {
        // (col_x, col_y, norm_x, norm_y, distance)

        let mut ret: Option<(f64, f64, f64, f64, f64)> = None;
        let mut min_dist = f64::INFINITY;

        // Calculate which tiles the ray passes through using DDA algorithm
        let todo = self.get_tiles_on_ray(start_x, start_y, dir_x, dir_y, map);

        for (tile_x, tile_y) in todo {
            if tile_x >= map.tiles_x || tile_y >= map.tiles_y {
                continue;
            }

            let tile_index = tile_y * map.tiles_x + tile_x;
            let tile = &map.tiles[tile_index];

            // Check each wall of the tile
            let walls = [
                (tile.up, 0),    // up wall
                (tile.right, 1), // right wall
                (tile.down, 2),  // down wall
                (tile.left, 3),  // left wall
            ];

            for (is_wall, wall_direction) in walls.iter() {
                if !is_wall {
                    continue;
                }

                let (wx1, wy1, wx2, wy2) =
                    map.wall_coords(tile_x as isize, tile_y as isize, *wall_direction);

                if let Some((collision_x, collision_y, distance)) = self.ray_line_intersection(
                    start_x, start_y, dir_x, dir_y, wx1 as f64, wy1 as f64, wx2 as f64, wy2 as f64,
                ) {
                    if distance > 0.0001 && distance < min_dist {
                        // Small epsilon to avoid self-collision
                        // Calculate wall normal
                        let (normal_x, normal_y) = match *wall_direction {
                            0 => (0.0, -1.0), // up
                            1 => (-1.0, 0.0), // right
                            2 => (0.0, 1.0),  // down
                            3 => (1.0, 0.0),  // left
                            _ => (0.0, 0.0),
                        };
                        min_dist = distance;
                        ret = Some((collision_x, collision_y, normal_x, normal_y, distance));
                    }
                }
            }
        }

        if ret.is_none() {
            // Log when no collision is found but we expect one
            kprintln!(
                "No collision found from ({}, {}) in direction ({}, {})",
                start_x,
                start_y,
                dir_x,
                dir_y
            );
        }

        ret
    }

    fn get_tiles_on_ray(
        &self,
        start_x: f64,
        start_y: f64,
        dir_x: f64,
        dir_y: f64,
        map: &Map,
    ) -> Vec<(usize, usize)> {
        let mut tiles = Vec::new();
        let tile_size = map.tile_size as f64;

        // Current position in tile coordinates
        let mut current_tile_x = (start_x / tile_size).floor() as isize;
        let mut current_tile_y = (start_y / tile_size).floor() as isize;

        // Direction of movement in tile space
        let step_x = if dir_x > 0.0 { 1 } else { -1 };
        let step_y = if dir_y > 0.0 { 1 } else { -1 };

        // Calculate delta distances
        let delta_dist_x = if dir_x != 0.0 {
            (1.0 / dir_x).abs()
        } else {
            f64::INFINITY
        };
        let delta_dist_y = if dir_y != 0.0 {
            (1.0 / dir_y).abs()
        } else {
            f64::INFINITY
        };

        // Calculate initial side distances
        let mut side_dist_x = if dir_x < 0.0 {
            (start_x / tile_size - current_tile_x as f64) * delta_dist_x
        } else {
            ((current_tile_x + 1) as f64 - start_x / tile_size) * delta_dist_x
        };

        let mut side_dist_y = if dir_y < 0.0 {
            (start_y / tile_size - current_tile_y as f64) * delta_dist_y
        } else {
            ((current_tile_y + 1) as f64 - start_y / tile_size) * delta_dist_y
        };

        // DDA algorithm - traverse tiles until we reach map boundaries
        let max_tiles = ((BULLET_TRAVEL_DIST / tile_size).ceil() as usize * 3).max(100);
        for _ in 0..max_tiles {
            // Add current tile if it's within bounds
            if current_tile_x >= 0
                && current_tile_y >= 0
                && current_tile_x < map.tiles_x as isize
                && current_tile_y < map.tiles_y as isize
            {
                tiles.push((current_tile_x as usize, current_tile_y as usize));
            } else {
                break; // Out of bounds
            }

            // Move to next tile
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                current_tile_x += step_x;
            } else {
                side_dist_y += delta_dist_y;
                current_tile_y += step_y;
            }
        }

        tiles
    }

    fn ray_line_intersection(
        &self,
        ray_start_x: f64,
        ray_start_y: f64,
        ray_dir_x: f64,
        ray_dir_y: f64,
        line_x1: f64,
        line_y1: f64,
        line_x2: f64,
        line_y2: f64,
    ) -> Option<(f64, f64, f64)> {
        // Ray: (ray_start_x, ray_start_y) + t * (ray_dir_x, ray_dir_y)
        // Line segment: (line_x1, line_y1) to (line_x2, line_y2)

        let dx_line = line_x2 - line_x1;
        let dy_line = line_y2 - line_y1;

        let dx_ray = ray_dir_x;
        let dy_ray = ray_dir_y;

        let determinant = dx_ray * dy_line - dy_ray * dx_line;
        if determinant.abs() < 1e-10 {
            // Lines are parallel
            return None;
        }

        let t =
            ((line_x1 - ray_start_x) * dy_line - (line_y1 - ray_start_y) * dx_line) / determinant;
        let u = ((line_x1 - ray_start_x) * dy_ray - (line_y1 - ray_start_y) * dx_ray) / determinant;

        if t >= 0.0 && u >= 0.0 && u <= 1.0 {
            let intersection_x = ray_start_x + t * dx_ray;
            let intersection_y = ray_start_y + t * dy_ray;
            let distance = ((intersection_x - ray_start_x).powi(2)
                + (intersection_y - ray_start_y).powi(2))
            .sqrt();
            Some((intersection_x, intersection_y, distance))
        } else {
            None
        }
    }

    fn ray_line_intersection_old(
        &self,
        ray_start_x: f64,
        ray_start_y: f64,
        ray_dir_x: f64,
        ray_dir_y: f64,
        line_x1: f64,
        line_y1: f64,
        line_x2: f64,
        line_y2: f64,
    ) -> Option<(f64, f64, f64)> {
        // Returns: (intersection_x, intersection_y, distance)

        let line_dir_x = line_x2 - line_x1;
        let line_dir_y = line_y2 - line_y1;

        let denominator = ray_dir_x * line_dir_y - ray_dir_y * line_dir_x;

        if denominator.abs() < 1e-10 {
            return None; // Lines are parallel
        }

        let dx = line_x1 - ray_start_x;
        let dy = line_y1 - ray_start_y;

        let t = (dx * line_dir_y - dy * line_dir_x) / denominator;
        let u = (dx * ray_dir_y - dy * ray_dir_x) / denominator;

        if t >= 0.0 && u >= 0.0 && u <= 1.0 {
            let intersection_x = ray_start_x + t * ray_dir_x;
            let intersection_y = ray_start_y + t * ray_dir_y;
            Some((intersection_x, intersection_y, t))
        } else {
            None
        }
    }

    pub fn update(&mut self, tick_delta: f64) -> bool {
        // Returns false if bullet should be remove

        let travel_distance = tick_delta * BULLET_SPEED;

        loop {
            // Calculate distance to current target
            let dx = self.pointing_towards_x - self.x;
            let dy = self.pointing_towards_y - self.y;
            let distance_to_target = (dx * dx + dy * dy).sqrt();

            if travel_distance <= distance_to_target {
                // Move towards current target
                if distance_to_target > 0.0 {
                    let move_x = (dx / distance_to_target) * travel_distance;
                    let move_y = (dy / distance_to_target) * travel_distance;
                    self.x += move_x;
                    self.y += move_y;
                }
                self.already_traveled += travel_distance;
                return self.already_traveled < BULLET_TRAVEL_DIST;
            } else {
                // Reach current target and move to next point
                self.x = self.pointing_towards_x;
                self.y = self.pointing_towards_y;
                self.already_traveled += distance_to_target;

                // Get next target from queue
                if let Some(next_point) = self.path_queue.pop_front() {
                    self.current_segment_start = (next_point.0, next_point.1);
                    self.pointing_towards_x = next_point.0;
                    self.pointing_towards_y = next_point.1;

                    // Continue with remaining travel distance
                    let remaining_travel = travel_distance - distance_to_target;
                    if remaining_travel <= 0.001 {
                        // Small epsilon
                        return self.already_traveled < BULLET_TRAVEL_DIST;
                    }
                    // Continue loop with remaining distance
                } else {
                    // No more path points, bullet has reached its end
                    return false;
                }
            }
        }
    }
}

impl Serializable for Bullet {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize only essential data for network transmission
        // Position (8 bytes each for x and y as f64)
        result.extend_from_slice(&self.x.to_le_bytes());
        result.extend_from_slice(&self.y.to_le_bytes());

        // Serialize pointing towards (8 bytes each as f64)
        result.extend_from_slice(&self.pointing_towards_x.to_le_bytes());
        result.extend_from_slice(&self.pointing_towards_y.to_le_bytes());

        // Serialize already_traveled (8 bytes)
        result.extend_from_slice(&self.already_traveled.to_le_bytes());

        // Don't serialize path_queue and current_segment_start to save space
        // Clients will recalculate the path locally if needed

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 40 {
            // Minimum size for essential fields only
            return Bullet {
                x: 0.0,
                y: 0.0,
                pointing_towards_x: 0.0,
                pointing_towards_y: 0.0,
                already_traveled: 0.0,
                current_segment_start: (0.0, 0.0),
                path_queue: VecDeque::new(),
            };
        }

        let mut offset = 0;

        // Deserialize position
        let x = f64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        let y = f64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        // Deserialize pointing towards
        let pointing_towards_x = f64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        let pointing_towards_y = f64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        offset += 8;

        // Deserialize already_traveled
        let already_traveled = f64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        // Create bullet with minimal data - path will be empty and needs recalculation
        Bullet {
            x,
            y,
            pointing_towards_x,
            pointing_towards_y,
            already_traveled,
            current_segment_start: (x, y), // Use current position as segment start
            path_queue: VecDeque::new(),   // Empty path queue - will be recalculated if needed
        }
    }
}

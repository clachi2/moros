use crate::kprintln;
use crate::usr::game::tanks::map::Map;
use crate::usr::game::tanks::state::{BULLET_SPEED, BULLET_TRAVEL_DIST, Serializable};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use num_traits::Float;

/// Represents a bullet in the game, including its position, direction, and path.
pub struct Bullet {
    pub x: f64,
    pub y: f64,
    pub pointing_towards_x: f64,
    pub pointing_towards_y: f64,
    pub already_traveled: f64,
    pub path_queue: VecDeque<(f64, f64)>,
    pub current_segment_start: (f64, f64),
    pub shot_by: usize, // Player ID who shot this bullet
}

impl Bullet {
    /// Creates a new instance of the `Bullet` struct, initializing its position, target, shooter,
    /// and entire calculated path (considering any reflections) based on the given map.
    pub fn new(
        start_x: f64,
        start_y: f64,
        target_x: f64,
        target_y: f64,
        shot_by: usize,
        map: &Map,
    ) -> Self {
        let mut bullet = Bullet {
            x: start_x,
            y: start_y,
            pointing_towards_x: target_x,
            pointing_towards_y: target_y,
            already_traveled: 0.0,
            path_queue: VecDeque::new(),
            current_segment_start: (start_x, start_y),
            shot_by,
        };

        // calc entire path with reflections
        bullet.calculate_path(map);
        bullet
    }

    /// Calculates the path of the Bullet on a provided `Map` and saves it in `self.path_queue`.
    ///
    /// The function determines the trajectory of the object by computing its path
    /// based on its current position, direction, and collisions with the map. The
    /// path is calculated up to a maximum travel distance (`BULLET_TRAVEL_DIST`) and
    /// is stored as a sequence of points in `self.path_queue`. If a collision is
    /// encountered, the object may change direction based on collision rules, and
    /// the path is updated accordingly.
    ///
    /// # Parameters
    /// - `map`: A reference to the `Map` object representing the walls and boundaries that
    /// the bullet can collide with.
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

        let mut already_traveled = self.already_traveled;
        while already_traveled < BULLET_TRAVEL_DIST {
            // Find next collision point
            if let Some((col_x, col_y, new_dir_x, new_dir_y, dist)) =
                self.find_next_collision(current_x, current_y, direction_x, direction_y, map)
            {
                // check if the bullet wont reach the next collision point
                if already_traveled + dist >= BULLET_TRAVEL_DIST {
                    // travel remaining distance to the collision point
                    let remaining_dist = BULLET_TRAVEL_DIST - already_traveled;
                    self.path_queue.push_back((
                        current_x + direction_x * remaining_dist,
                        current_y + direction_y * remaining_dist,
                    ));
                    break;
                }

                // Collision found, update position and direction
                self.path_queue.push_back((col_x, col_y));
                (current_x, current_y) = self.clamp_to_map_bounds_and_round(col_x, col_y, map);
                already_traveled += dist;
                direction_x = new_dir_x;
                direction_y = new_dir_y;
            } else {
                // no collision found -> bullet travels in a straight line until max distance
                let remaining_dist = BULLET_TRAVEL_DIST - already_traveled;
                let final_x = current_x + direction_x * remaining_dist;
                let final_y = current_y + direction_y * remaining_dist;
                self.path_queue.push_back((final_x, final_y));
                break;
            }
        }

        // dequeue first point and set it as the current target
        if let Some(first_point) = self.path_queue.front() {
            self.pointing_towards_x = first_point.0;
            self.pointing_towards_y = first_point.1;
        }
    }

    /// Clamps the given x and y coordinates to the bounds of the provided map and rounds them to
    /// 8 decimal places to ensure precision.
    ///
    /// # Parameters
    /// - `x` (`f64`): The x-coordinate to be clamped and rounded.
    /// - `y` (`f64`): The y-coordinate to be clamped and rounded.
    /// - `map` (`&Map`): A reference to the map object containing the information about the map's
    ///   tile dimensions and sizes.
    ///
    /// # Returns
    /// - `(f64, f64)`: A tuple containing the clamped and rounded x and y coordinates.
    fn clamp_to_map_bounds_and_round(&self, x: f64, y: f64, map: &Map) -> (f64, f64) {
        let max_x = (map.tiles_x * map.tile_size) as f64;
        let max_y = (map.tiles_y * map.tile_size) as f64;

        let clamped_x = x.max(0.0).min(max_x);
        let clamped_y = y.max(0.0).min(max_y);

        let rounded_x = (clamped_x * 1e8).round() / 1e8;
        let rounded_y = (clamped_y * 1e8).round() / 1e8;

        (rounded_x, rounded_y)
    }

    ///
    /// Finds the next collision of a ray with walls on a grid-based map, given a start position, direction, and map information.
    ///
    /// The method uses a DDA (Digital Differential Analyzer) algorithm to traverse tiles along the ray's path,
    /// checking each tile for potential collisions with walls. If a collision occurs, it calculates the collision point,
    /// the normal vector of the surface the ray strikes, and the distance from the start to the collision.
    /// Additionally, it computes the ray's reflected direction based on the wall's normal vector.
    ///
    /// ### Parameters:
    /// - `start_x`: `f64` - The starting `x` coordinate of the ray.
    /// - `start_y`: `f64` - The starting `y` coordinate of the ray.
    /// - `dir_x`: `f64` - The `x` direction component of the ray.
    /// - `dir_y`: `f64` - The `y` direction component of the ray.
    /// - `map`: `&Map` - A reference to the map object representing the grid and its walls.
    ///
    /// ### Returns:
    /// - `Option<(f64, f64, f64, f64, f64)>`:
    ///   - `Some((collision_x, collision_y, reflected_dir_x, reflected_dir_y, distance))`:
    ///     - `collision_x`: `f64` - The `x` coordinate of the collision point.
    ///     - `collision_y`: `f64` - The `y` coordinate of the collision point.
    ///     - `reflected_dir_x`: `f64` - The `x` component of the reflected ray's direction.
    ///     - `reflected_dir_y`: `f64` - The `y` component of the reflected ray's direction.
    ///     - `distance`: `f64` - The distance from the starting point to the collision.
    ///   - `None`: If no collision is found.
    fn find_next_collision(
        &self,
        start_x: f64,
        start_y: f64,
        dir_x: f64,
        dir_y: f64,
        map: &Map,
    ) -> Option<(f64, f64, f64, f64, f64)> {
        // (col_x, col_y, dir_x,dir_y, distance)

        let mut ret: Option<(f64, f64, f64, f64, f64)> = None;
        let mut min_dist = f64::INFINITY;

        // Calculate which tiles the ray passes through using DDA algorithm
        let todo = self.get_tiles_on_ray(start_x, start_y, dir_x, dir_y, map);

        // Check each tile for collisions
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

                // Get wall coordinates based on tile position and direction
                let (wx1, wy1, wx2, wy2) =
                    map.wall_coords(tile_x as isize, tile_y as isize, *wall_direction);

                // Check for intersection with the ray
                if let Some((collision_x, collision_y, distance)) = self.ray_line_intersection(
                    start_x, start_y, dir_x, dir_y, wx1 as f64, wy1 as f64, wx2 as f64, wy2 as f64,
                ) {
                    if distance > 0.0001 && distance < min_dist {
                        // Small epsilon to avoid self-collision
                        // Calculate wall normal for reflection
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
            None
        } else {
            // Calculate reflected direction based on the wall normal
            let dot = dir_x * ret.unwrap().2 + dir_y * ret.unwrap().3;
            let new_dir_x = dir_x - 2.0 * dot * ret.unwrap().2;
            let new_dir_y = dir_y - 2.0 * dot * ret.unwrap().3;
            ret = Some((
                ret.unwrap().0,
                ret.unwrap().1,
                new_dir_x,
                new_dir_y,
                ret.unwrap().4,
            ));
            ret
        }
    }

    /// Computes the tiles that a ray intersects as it traverses a 2D grid map.
    ///
    /// This function uses a Digital Differential Analyzer (DDA) algorithm to
    /// calculate the tiles that a ray intersects, starting from a given point
    /// and traveling in a specific direction. The ray traversal takes into
    /// account tile boundaries within the map limits.
    ///
    /// # Parameters
    /// - `&self`: Borrowed reference to the instance of the struct containing this method.
    /// - `start_x`: The starting x-coordinate of the ray in world space.
    /// - `start_y`: The starting y-coordinate of the ray in world space.
    /// - `dir_x`: The x-component of the direction vector of the ray.
    /// - `dir_y`: The y-component of the direction vector of the ray.
    /// - `map`: A reference to the `Map` object, which provides information about the grid
    ///   structure
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

    /// Computes the intersection point of a ray and a line segment, if one exists.
    ///
    /// # Returns
    /// - `Some((intersection_x, intersection_y, distance))` if there is an intersection:
    ///   - `intersection_x`: The x-coordinate of the intersection point.
    ///   - `intersection_y`: The y-coordinate of the intersection point.
    ///   - `distance`: The distance from the ray's starting point to the intersection point.
    /// - `None` if there is no intersection or the ray and line segment are parallel.
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

    /// Updates the position and state of the bullet.
    ///
    /// The `update` method moves the bullet along its predetermined path based on
    /// the time delta (`tick_delta`) and its speed. The bullet continues traveling
    /// from its current position (`self.x`, `self.y`) towards its current target point
    /// (`self.pointing_towards_x`, `self.pointing_towards_y`), following the path
    /// defined in `self.path_queue`. During its motion, the bullet may either move
    /// further along the path or stop if it has reached the end of its travel distance.
    ///
    /// # Parameters
    /// - `tick_delta`: A `f64` value indicating the time interval since the last update.
    ///                 This value determines how far the bullet travels in the current frame.
    ///
    /// # Returns
    /// A `bool` indicating whether the bullet should remain active (`true`) or be
    /// removed (`false`).
    pub fn update(&mut self, tick_delta: f64) -> bool {
        // Returns false if bullet should be remove

        let mut travel_distance = tick_delta * BULLET_SPEED;

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
                    travel_distance -= distance_to_target;
                    if travel_distance <= 0.001 {
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
    /// Serializes the `Bullet` instance into a byte vector.
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize only essential data for network transmission
        // Position (4 bytes each for x and y as f32)
        result.extend_from_slice(&(self.x as f32).to_le_bytes());
        result.extend_from_slice(&(self.y as f32).to_le_bytes());

        // Serialize pointing towards (4 bytes each as f32)
        result.extend_from_slice(&(self.pointing_towards_x as f32).to_le_bytes());
        result.extend_from_slice(&(self.pointing_towards_y as f32).to_le_bytes());

        // Serialize already_traveled (4 bytes as f32)
        result.extend_from_slice(&(self.already_traveled as f32).to_le_bytes());

        // Serialize shot_by (4 bytes as u32)
        result.extend_from_slice(&(self.shot_by as u32).to_le_bytes());

        // Don't serialize path_queue and current_segment_start to save space
        // Clients will recalculate the path locally if needed

        result
    }

    /// Deserializes a byte slice into a `Bullet` instance.
    ///
    /// If the data is too short, it returns a default `Bullet` with minimal data.
    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 24 {
            // Minimum size for essential fields only (5 f32 values + 1 u32 value)
            return Bullet {
                x: 0.0,
                y: 0.0,
                pointing_towards_x: 0.0,
                pointing_towards_y: 0.0,
                already_traveled: 0.0,
                current_segment_start: (0.0, 0.0),
                path_queue: VecDeque::new(),
                shot_by: 0,
            };
        }

        let mut offset = 0;

        // Deserialize position (f32 to f64)
        let x = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as f64;
        offset += 4;

        let y = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as f64;
        offset += 4;

        // Deserialize pointing towards (f32 to f64)
        let pointing_towards_x = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as f64;
        offset += 4;

        let pointing_towards_y = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as f64;
        offset += 4;

        // Deserialize already_traveled (f32 to f64)
        let already_traveled = f32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as f64;
        offset += 4;

        // Deserialize shot_by (u32 to usize)
        let shot_by = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        // Create bullet with minimal data - path will be empty and needs recalculation
        Bullet {
            x,
            y,
            pointing_towards_x,
            pointing_towards_y,
            already_traveled,
            current_segment_start: (x, y), // Use current position as segment start
            path_queue: VecDeque::new(),   // Empty path queue - will be recalculated if needed
            shot_by,
        }
    }
}

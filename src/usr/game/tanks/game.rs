use crate::api::process::ExitCode;
use crate::sys::clk::boot_time;
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::tanks::bullet::Bullet;
use crate::usr::game::tanks::map::Map;
use crate::usr::game::tanks::player::Player;
use crate::usr::game::tanks::renderer;
use crate::usr::game::tanks::renderer::Color;
use crate::usr::game::tanks::state;
use crate::usr::game::tanks::state::{
    GUI_WIDTH, MAX_AMMO, PLAYER_SIZE, POINTS_PER_DEATH_MINUS, POINTS_PER_KILL, RESPAWN_TIME,
    SHOOTING_RATE_PER_SECOND, Serializable, TICK_RATE, WALL_DENSITY,
};
use crate::usr::game::tanks::userinput::UserInput;
use alloc::vec::Vec;

pub static COLOR_DEPTH: u8 = 8;
pub static TILES_X: usize = 12;
pub static TILES_Y: usize = 10;
pub static TILE_SIZE: usize = 20;

/// Main game structure containing the game state and renderer.
pub struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
}

impl Game {
    /// Creates a new `Game` instance with the specified width and height.
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = Map::new(width, height, TILES_X, TILES_Y, TILE_SIZE);
        map.auto_set_walls();
        let game_state = state::GameState::new(map);
        let renderer =
            renderer::Renderer::new(width, height, COLOR_DEPTH, TILES_X, TILES_Y, TILE_SIZE);
        Game {
            game_state,
            renderer,
        }
    }

    /// Initializes the game, setting up the map and renderer.
    pub fn init(&mut self) {
        self.game_state.map.auto_set_walls();
        self.renderer.init();
        self.renderer.draw_map_buffer(self.game_state.map.clone());
    }

    /// Deinitializes the game, cleaning up resources.
    pub fn deinit(&mut self) {
        self.renderer.deinit();
    }

    /// Updates the game state on every tick, advancing the simulation forward based on the time elapsed.
    ///
    /// This method performs critical game logic, including updating entity positions, handling interactions,
    /// and ensuring the game runs consistently at the defined tick rate.
    ///
    /// # Functionality
    /// - **Tick Timing:**
    ///   - Calculates the time elapsed since the last tick using the `boot_time()`.
    ///   - Ensures that ticks only progress when enough time has passed according to the defined `TICK_RATE`.
    ///   - If insufficient time has passed, the method returns early without processing further updates.
    ///   - Updates the `last_tick` timestamp after completing the tick calculations.
    ///
    /// - **Player Updates:**
    ///   - Iterates over all players in the game.
    ///   - For players marked as `alive`, their positions are updated using the elapsed time (`tick_delta`) and the game map.
    ///
    /// - **Player Shooting:**
    ///   - Processes any ongoing or newly initiated shooting actions for players based on the current game state and time.
    ///
    /// - **Bullet Updates:**
    ///   - Updates the positions of all bullets in the game based on their movement logic and elapsed time.
    ///   - Removes bullets that are no longer active (e.g., if they collided or exceeded their range).
    ///
    /// - **Collision Detection:**
    ///   - Handles collisions between players and bullets.
    ///   - Adjusts the game state when a collision is detected, such as updating player health, marking players/bullets as inactive, or generating effects.
    ///
    /// - **Player Respawning:**
    ///   - Checks whether dead players should respawn (based on time or other conditions).
    ///   - Handles the necessary state changes to bring a player back into the game post-respawn.
    ///
    /// # Notes
    /// - This function assumes that `TICK_RATE` is predefined and determines the frequency of game ticks.
    /// - It also expects the game state (`self.game_state`) to contain valid references to players, bullets, and the map.
    ///
    /// # Usage
    /// Call this method on every frame or at regular intervals to ensure the game progresses consistently as intended.
    pub fn tick(&mut self) {
        // Tick timing
        let current_time = boot_time();
        let tick_delta = current_time - self.game_state.last_tick;
        if tick_delta < 1.0 / TICK_RATE {
            return; // Not enough time has passed for the next tick
        }
        self.game_state.last_tick = current_time;

        // update Player positions
        for player in &mut self.game_state.players {
            if player.alive {
                player.update_position(tick_delta, &self.game_state.map);
            }
        }

        // Handle Player shooting
        self.handle_player_shooting(current_time);

        // Update Bullet positions
        self.game_state
            .bullets
            .retain_mut(|bullet| bullet.update(tick_delta));

        // Check Collisions between Players and Bullets
        self.handle_player_bullet_collisions(current_time);

        // Check if dead players need to respawn
        self.handle_player_respawning(current_time);
    }

    /// Handles player shooting logic, including rate limiting and ammo management.
    ///
    /// This function processes each player's shooting input, creates bullets,
    /// and manages ammo reloading over time
    ///
    /// # Parameters
    /// - `current_time`: The current time in seconds, used for rate limiting and reload
    fn handle_player_shooting(&mut self, current_time: f64) {
        for player in &mut self.game_state.players {
            if !player.alive {
                continue; // Skip dead players
            }
            // Handle Player shooting
            if player.user_input.shooting && player.ammo > 0 {
                // Check if enough time has passed since last shot (rate limiting)
                let time_since_last_shot = current_time - player.last_shot;
                let min_shot_interval = 1.0 / SHOOTING_RATE_PER_SECOND;

                if time_since_last_shot >= min_shot_interval {
                    let target_x = (player.user_input.map_mouse_x - GUI_WIDTH as isize) as f64;
                    let target_y = player.user_input.map_mouse_y as f64;

                    let bullet = Bullet::new(
                        player.x + (PLAYER_SIZE as f64 / 2.0), // Center of player
                        player.y + (PLAYER_SIZE as f64 / 2.0),
                        target_x,
                        target_y,
                        player.id,
                        &self.game_state.map,
                    );
                    self.game_state.bullets.push(bullet);

                    player.ammo -= 1;
                    player.last_shot = current_time;
                }
            }
            // reload ammo
            else if player.ammo < state::MAX_AMMO {
                let time_since_last_reload = current_time - player.last_reload_ammo;
                if time_since_last_reload >= state::RELOAD_TIME {
                    player.ammo += 1;
                    player.last_reload_ammo = current_time;
                }
            }
        }
    }

    /// Handles collisions between players and bullets, updating player states and awarding points.
    /// This function checks for collisions, marks players as dead, removes bullets,
    /// and awards points to players who successfully hit others.
    /// # Parameters
    /// - `current_time`: The current time in seconds, used for updating player
    fn handle_player_bullet_collisions(&mut self, current_time: f64) {
        let mut bullets_to_remove = Vec::new();
        let mut player_ids_to_award = Vec::new();

        for (bullet_idx, bullet) in self.game_state.bullets.iter().enumerate() {
            for player in self.game_state.players.iter_mut() {
                if player.alive
                    && bullet.shot_by != player.id // bullet cant hit its owner
                    && bullet.x < player.x + PLAYER_SIZE as f64
                    && bullet.x > player.x
                    && bullet.y < player.y + PLAYER_SIZE as f64
                    && bullet.y > player.y
                {
                    player.alive = false;
                    player.time_of_death = current_time;
                    player.points = player.points.saturating_sub(POINTS_PER_DEATH_MINUS); // Decrease points on death
                    player_ids_to_award.push(bullet.shot_by);

                    bullets_to_remove.push(bullet_idx);
                    break; // One bullet can only hit one player
                }
            }
        }

        // Award points to players who shot the bullets that hit players
        for player in &mut self.game_state.players {
            if player_ids_to_award.contains(&player.id) {
                player.points += POINTS_PER_KILL;
            }
        }

        // Remove bullets that hit players (in reverse order to maintain indices)
        for &bullet_idx in bullets_to_remove.iter().rev() {
            self.game_state.bullets.remove(bullet_idx);
        }
    }

    /// Handles the respawning of dead players after a set respawn time.
    /// This function checks each player's status and respawns them if the
    /// required time has passed since their death.
    /// # Parameters
    /// - `current_time`: The current time in seconds, used to determine if a player
    fn handle_player_respawning(&mut self, current_time: f64) {
        for player in &mut self.game_state.players {
            if !player.alive && (current_time - player.time_of_death) >= RESPAWN_TIME {
                // Respawn player
                let pos = self.game_state.map.random_pos();
                player.x = pos.0 as f64;
                player.y = pos.1 as f64;
                player.alive = true;
                player.ammo = 5; // Reset ammo
                player.time_of_death = 0.0;
            }
        }
    }

    /// Renders the current game state to the screen.
    /// This method clears the screen, draws the map, players, bullets,
    /// player stats, and the mouse cursor for the current player.
    /// It then flushes the renderer to update the display.
    pub fn draw(&mut self) {
        self.renderer.clear();

        self.renderer.draw_map();

        // Draw players
        for player in &self.game_state.players {
            if player.alive {
                self.renderer.draw_player(
                    player.x as isize + GUI_WIDTH as isize,
                    player.y as isize,
                    player.id as u8,
                );
            }
        }

        // Draw bullets
        for bullet in &self.game_state.bullets {
            self.renderer.draw_bullet(
                bullet.x as isize + GUI_WIDTH as isize,
                bullet.y as isize,
                bullet.shot_by as u8,
            );
        }

        // Draw player stats
        for (index, player) in self.game_state.players.iter().enumerate() {
            self.renderer
                .draw_stat(index, player.id, player.points, player.ammo);
        }

        // Draw mouse cursor for the current player
        if self.game_state.current_player_index < self.game_state.players.len() {
            let player = &self.game_state.players[self.game_state.current_player_index];
            //kprintln!("player id {} with index {}", player.id, self.game_state.current_player_index);
            let mouse_x = player.user_input.map_mouse_x;
            let mouse_y = player.user_input.map_mouse_y;
            self.renderer
                .draw_mouse_cursor(mouse_x, mouse_y, player.id as u8);
        }

        self.renderer.flush();
    }

    /// Adds a new player to the game.
    /// If a player with the same ID already exists, it does not add a duplicate.
    /// The new player is initialized at a random position on the map with default attributes.
    /// # Parameters
    /// - `player_id`: The unique identifier for the new player.
    pub fn add_player(&mut self, player_id: usize) {
        // Check if player already exists
        for player in &self.game_state.players {
            if player.id == player_id {
                kprintln!(
                    "Player with ID {} already exists, not adding again",
                    player_id
                );
                return; // Player already exists, do not add again
            }
        }

        // Create a new player at a random position on the map
        let pos = self.game_state.map.random_pos();
        let new_player = Player {
            id: player_id,
            x: pos.0 as f64,
            y: pos.1 as f64,
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
            ammo: MAX_AMMO,
            last_shot: 0.0,
            last_reload_ammo: 0.0,
        };
        self.game_state.players.push(new_player);
    }

    /// Removes a player from the game based on their unique identifier.
    /// If the player with the specified ID is found, they are removed from the game state
    /// and a message is logged. If no such player exists, a different message is logged.
    /// # Parameters
    /// - `player_id`: The unique identifier of the player to be removed.
    pub fn remove_player(&mut self, player_id: usize) {
        // Find the player index by ID
        if let Some(index) = self
            .game_state
            .players
            .iter()
            .position(|p| p.id == player_id)
        {
            // Remove the player from the game state
            self.game_state.players.remove(index);
            //kprintln!("Player with ID {} removed", player_id);
        } else {
            //kprintln!("Player with ID {} not found", player_id);
        }
    }

    /// Updates the user input for a specific player based on the provided width and height.
    /// This function searches for the player with the given `player_id` and updates their
    /// `user_input` field by calling the `update_from_io` method with the specified
    /// width and height parameters.
    /// If the player is found, their input is updated; otherwise, a message is logged
    /// indicating that the player was not found.
    /// # Parameters
    /// - `player_id`: The unique identifier of the player whose input is to be updated
    pub fn update_user_input(&mut self, player_id: usize, witdth: usize, height: usize) {
        for player in &mut self.game_state.players {
            if player.id == player_id {
                player.user_input.update_from_io(witdth, height);
                return; // User input set successfully
            }
        }
        kprintln!("Player with ID {} not found", player_id);
    }

    /// Serializes the user input of a specific player into a byte vector.
    /// This function searches for the player with the given `player_id` and
    /// serializes their `user_input` field using the `serialize` method.
    /// If the player is found, their serialized input is returned; otherwise,
    /// an empty vector is returned.
    /// # Parameters
    /// - `player_id`: The unique identifier of the player whose input is to be serialized
    /// # Returns
    /// A vector of bytes representing the serialized user input of the specified player.
    /// If the player is not found, an empty vector is returned.
    pub fn serialize_user_input(&self, player_id: usize) -> Vec<u8> {
        for player in &self.game_state.players {
            if player.id == player_id {
                return player.user_input.serialize();
            }
        }
        Vec::new()
    }

    /// Deserializes and updates the user input for a specific player based on the provided byte slice.
    /// This function searches for the player with the given `player_id` and updates their
    /// `user_input` field by calling the `deserialize` method with the provided byte slice
    /// `data`. If the player is found, their input is updated; otherwise,
    /// a message is logged indicating that the player was not found.
    /// # Parameters
    /// - `player_id`: The unique identifier of the player whose input is to be updated
    /// - `data`: A byte slice containing the serialized user input data to be deserialized
    pub fn deserialize_user_input(&mut self, player_id: usize, data: &[u8]) {
        for player in &mut self.game_state.players {
            if player.id == player_id {
                player.user_input = UserInput::deserialize(data);
                return; // User input set successfully
            }
        }
        kprintln!("Player with ID {} not found", player_id);
    }

    /// Serializes the entire game state into a byte vector.
    pub fn serialize_state(&self) -> Vec<u8> {
        self.game_state.serialize()
    }

    /// Deserializes the game state from a byte slice and updates the current game state.
    /// It preserves the last tick time and the current player index to maintain continuity.
    /// After updating the game state, it also refreshes the renderer with the new map.
    /// # Parameters
    /// - `data`: A byte slice containing the serialized game state data to be deserialized
    pub fn deserialize_state(&mut self, data: &[u8]) {
        let last_tick = self.game_state.last_tick;
        let player_index = self.game_state.current_player_index;

        self.game_state = state::GameState::deserialize(data);
        // Restore the last tick time
        self.game_state.last_tick = last_tick;
        // Restore the current player index
        self.game_state.current_player_index = player_index;
        // Update the renderer with the new map
        self.renderer.draw_map_buffer(self.game_state.map.clone());
    }

    /// Serializes the current game map into a byte vector.
    pub fn serialize_map(&self) -> Vec<u8> {
        self.game_state.map.serialize()
    }

    /// Deserializes the game map from a byte slice and updates the current game map.
    /// After updating the map, it also refreshes the renderer with the new map.
    /// # Parameters
    /// - `data`: A byte slice containing the serialized map data to be deserialized
    pub fn deserialize_map(&mut self, data: &[u8]) {
        let map = Map::deserialize(data);
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    /// Sets the current player based on the provided player ID.
    /// This function searches for the player with the given `player_id` and updates
    /// the `current_player_index` in the game state to point to that player.
    /// If the player is found, the current player index is updated; otherwise,
    /// no changes are made.
    /// # Parameters
    /// - `player_id`: The unique identifier of the player to be set as the current
    pub fn set_current_player(&mut self, player_id: usize) {
        // Find the player index by ID
        for (index, player) in self.game_state.players.iter().enumerate() {
            if player.id == player_id {
                self.game_state.current_player_index = index;
                break;
            }
        }
    }

    /// Generates a new random map with the specified width and height,
    /// updates the game state with the new map, and refreshes the renderer.
    /// # Parameters
    /// - `width`: The width of the new map in pixels
    /// - `height`: The height of the new map in pixels
    pub fn set_new_random_map(&mut self, width: usize, height: usize) {
        let mut new_map = Map::new(width, height, TILES_X, TILES_Y, TILE_SIZE);
        new_map.auto_set_walls();
        self.game_state.map = new_map.clone();
        self.renderer.draw_map_buffer(new_map);
    }
}

/// A demo function that showcases the map generation and area connection algorithm.
/// It generates a random map with walls, finds distinct areas, and connects them by removing walls.
/// The demo waits for user input (mouse click) to proceed through each step of the process.
/// # Parameters
/// - `width`: The width of the display area for the demo
/// - `height`: The height of the display area for the demo
/// # Returns
/// - `Ok(())` if the demo completes successfully
/// - `Err(ExitCode)` if an error occurs during the demo
pub fn map_demo(width: usize, height: usize) -> Result<(), ExitCode> {
    let mut map = Map::new(320, 200, TILES_X, TILES_Y, TILE_SIZE);
    let mut renderer =
        renderer::Renderer::new(width, height, COLOR_DEPTH, TILES_X, TILES_Y, TILE_SIZE);
    renderer.init();
    renderer.clear();

    // generate random walls
    map.clear_walls();
    map.add_random_walls(WALL_DENSITY);
    map.set_outer_walls();
    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
    renderer.flush();
    wait_until_pressend_and_released(width, height);

    let mut areas = map.find_distinct_areas();
    // areas.sort_by(|a, b| a.len().cmp(&b.len())); // Sort areas by size (smallest first)

    while areas.len() > 1 {
        renderer.draw_map_buffer(map.clone());
        renderer.draw_map();
        renderer.draw_areas(&areas);
        renderer.flush();
        wait_until_pressend_and_released(width, height);
        for area in areas.iter() {
            for tile_coord in area {
                let x = tile_coord.0;
                let y = tile_coord.1;
                let index = y * map.tiles_x + x;
                let index_right = y * map.tiles_x + (x + 1);
                let index_down = (y + 1) * map.tiles_x + x;
                if y > 0 && map.tiles[index].up && !area.contains(&(tile_coord.0, tile_coord.1 - 1))
                {
                    map.tiles[index].up = false;
                    let line = map.wall_coords(x as isize, y as isize, 0);
                    renderer.framebuffer.draw_line(
                        line.0 + GUI_WIDTH as isize,
                        line.1,
                        line.2 + GUI_WIDTH as isize,
                        line.3,
                        Color::Red as u8,
                    );
                    break;
                }
                if x < map.tiles_x - 1
                    && (map.tiles[index].right || map.tiles[index_right].left)
                    && !area.contains(&(tile_coord.0 + 1, tile_coord.1))
                {
                    map.tiles[index].right = false;
                    map.tiles[index_right].left = false;
                    let line = map.wall_coords(x as isize, y as isize, 1);
                    renderer.framebuffer.draw_line(
                        line.0 + GUI_WIDTH as isize + 1,
                        line.1,
                        line.2 + GUI_WIDTH as isize + 1,
                        line.3,
                        Color::Red as u8,
                    );
                    break;
                }
                if y < map.tiles_y - 1
                    && (map.tiles[index].down || map.tiles[index_down].up)
                    && !area.contains(&(tile_coord.0, tile_coord.1 + 1))
                {
                    map.tiles[index].down = false;
                    map.tiles[index_down].up = false;
                    let line = map.wall_coords(x as isize, y as isize, 2);
                    renderer.framebuffer.draw_line(
                        line.0 + GUI_WIDTH as isize,
                        line.1 + 1,
                        line.2 + GUI_WIDTH as isize,
                        line.3 + 1,
                        Color::Red as u8,
                    );
                    break;
                }
                if x > 0
                    && map.tiles[index].left
                    && !area.contains(&(tile_coord.0 - 1, tile_coord.1))
                {
                    map.tiles[index].left = false;
                    let line = map.wall_coords(x as isize, y as isize, 3);
                    renderer.framebuffer.draw_line(
                        line.0 + GUI_WIDTH as isize,
                        line.1,
                        line.2 + GUI_WIDTH as isize,
                        line.3,
                        Color::Red as u8,
                    );
                    break;
                }
            }
        }
        renderer.flush();
        wait_until_pressend_and_released(width, height);
        areas = map.find_distinct_areas();
    }

    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
    renderer.draw_areas(&areas);
    renderer.flush();
    wait_until_pressend_and_released(width, height);

    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
    renderer.flush();
    wait_until_pressend_and_released(width, height);

    renderer.deinit();

    Ok(())
}

/// Waits until the mouse is pressed and then released. (This is only used in the map demo)
/// This function continuously checks the mouse input state and only returns
/// after a complete press-and-release cycle is detected.
/// # Parameters
/// - `width`: The width of the boundary or map, used to clamp mouse input to valid x-coordinates
/// - `height`: The height of the boundary or map, used to clamp mouse input to valid y-coordinates
fn wait_until_pressend_and_released(width: usize, height: usize) {
    let mut input = UserInput {
        up: false,
        right: false,
        down: false,
        left: false,
        shooting: false,
        map_mouse_x: 0,
        map_mouse_y: 0,
    };
    get_mouse_buffer().clear_events();
    while !input.shooting {
        input.update_from_io(width, height);
    }
    while input.shooting {
        input.update_from_io(width, height);
    }
}

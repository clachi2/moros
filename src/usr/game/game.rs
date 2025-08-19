use crate::sys::clk::boot_time;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::Player;
use crate::usr::game::renderer;
use crate::usr::game::state;
use crate::usr::game::state::{GUI_WIDTH, MAX_AMMO, PLAYER_SIZE, POINTS_PER_DEATH_MINUS, POINTS_PER_KILL, Serializable, SHOOTING_RATE_PER_SECOND, TICK_RATE, RESPAWN_TIME};
use alloc::vec::Vec;

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

pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = Map::new(width, height, 12, 10, 20);
        map.auto_set_walls();
        let game_state = state::GameState::new(map);
        let renderer = renderer::Renderer::new(width, height, 8, 12, 10, 20);
        Game {
            game_state,
            renderer,
        }
    }

    pub fn init(&mut self) {
        self.game_state.map.auto_set_walls();
        self.renderer.init();
        self.renderer.draw_map_buffer(self.game_state.map.clone());
    }

    pub fn deinit(&mut self) {
        self.renderer.deinit();
    }

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
            self.renderer.draw_mouse_cursor(mouse_x, mouse_y, player.id as u8);
        }

        self.renderer.flush();
    }

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

    pub fn remove_player(&mut self, player_id: usize) {
        // Find the player index by ID
        if let Some(index) = self.game_state.players.iter().position(|p| p.id == player_id) {
            // Remove the player from the game state
            self.game_state.players.remove(index);
            //kprintln!("Player with ID {} removed", player_id);
        } else {
            //kprintln!("Player with ID {} not found", player_id);
        }
    }

    pub fn set_and_draw_map(&mut self, map: Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_user_input(&mut self, player_id: usize, input: UserInput) {
        for player in &mut self.game_state.players {
            if player.id == player_id {
                player.user_input = input;
                return; // User input set successfully
            }
        }
        // If player not found, you might want to handle this case
        kprintln!("Player with ID {} not found", player_id);
    }

    pub fn deserialize_user_input(&mut self, player_id: usize, data: &[u8]) {
        let user_input = UserInput::deserialize(data);
        self.set_user_input(player_id, user_input);
    }

    pub fn serialize_state(&self) -> Vec<u8> {
        self.game_state.serialize()
    }

    pub fn serialize_map(&self) -> Vec<u8> {
        self.game_state.map.serialize()
    }

    pub fn deserialize_state(&mut self, data: &[u8]) {
        //todo anders schicken
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

    pub fn deserialize_map(&mut self, data: &[u8]) {
        let map = Map::deserialize(data);
        self.set_and_draw_map(map);
    }

    pub fn get_random_spawn_position(&self) -> (usize, usize) {
        self.game_state.map.random_pos()
    }

    pub fn serialize_user_input(&self, player_id: usize) -> Vec<u8> {
        for player in &self.game_state.players {
            if player.id == player_id {
                return player.user_input.serialize();
            }
        }
        Vec::new()
    }

    pub fn get_player_count(&self) -> usize {
        self.game_state.players.len()
    }

    pub fn get_bullet_count(&self) -> usize {
        self.game_state.bullets.len()
    }

    pub fn set_current_player(&mut self, player_id: usize) {
        // Find the player index by ID
        for (index, player) in self.game_state.players.iter().enumerate() {
            if player.id == player_id {
                self.game_state.current_player_index = index;
                break;
            }
        }
    }
}

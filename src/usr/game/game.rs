use crate::sys::clk::boot_time;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::{Player, UserInput};
use crate::usr::game::renderer;
use crate::usr::game::state;
use crate::usr::game::state::{
    GUI_WIDTH, PLAYER_SIZE, POINTS_PER_DEATH_MINUS, POINTS_PER_KILL, SHOOTING_RATE_PER_SECOND,
    TICK_RATE,
};
use alloc::vec::Vec;

pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = Map::new(width, height, 12, 10, 20); // Example dimensions, adjust as needed
        map.auto_set_walls();
        let game_state = state::GameState::new(map);
        let renderer = renderer::Renderer::new(width, height, 8, 12, 10, 20);
        Game {
            game_state,
            renderer,
        }
    }

    pub fn init(&mut self) {
        self.renderer.init();
        self.renderer.draw_map_buffer(self.game_state.map.clone());

        let pos = self.game_state.map.random_pos();

        // tests
        self.game_state.players.push(Player {
            id: 2,
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
            ammo: 5,
            last_shot: 0.0,
            last_reload_ammo: 0.0,
        });
        let pos = self.game_state.map.random_pos();
        self.game_state.players.push(Player {
            id: 3,
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
            ammo: 5,
            last_shot: 0.0,
            last_reload_ammo: 0.0,
        });
    }

    pub fn deinit(&mut self) {
        self.renderer.deinit();
    }

    pub fn tick(&mut self) {
        // TODO update game state, handle input, etc.

        // Tick timing
        let current_time = boot_time();
        let tick_delta = current_time - self.game_state.last_tick;
        if tick_delta < 1.0 / TICK_RATE {
            return; // Not enough time has passed for the next tick
        }
        self.game_state.last_tick = current_time;

        for player in &mut self.game_state.players {
            // update Player positions
            if player.alive {
                // new wanted position based on movement direction and tick delta
                let new_pos = player.next_wanted_position(tick_delta);
                // TODO check for each pixel on line between current position and new position
                // check x and y movement separately because example on tablet
                if !self
                    .game_state
                    .map
                    .is_pos_colliding(player.x as isize, new_pos.1 as isize)
                {
                    // move vertically if no collision
                    player.y = new_pos.1;
                }
                if !self
                    .game_state
                    .map
                    .is_pos_colliding(new_pos.0 as isize, player.y as isize)
                {
                    // move horizontally if no collision
                    player.x = new_pos.0;
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

        // Update Bullet positions
        self.game_state
            .bullets
            .retain_mut(|bullet| bullet.update(tick_delta));

        // Check Collisions between Players and Bullets
        self.handle_player_bullet_collisions();

        // Check if dead players need to respawn
        self.handle_player_respawning(current_time);
    }

    fn handle_player_bullet_collisions(&mut self) {
        let mut bullets_to_remove = Vec::new();
        // let mut players_to_kill = Vec::new();

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
                    let current_time = boot_time();
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
        let respawn_time = 3.0; // 3 seconds respawn delay

        for player in &mut self.game_state.players {
            if !player.alive && (current_time - player.time_of_death) >= respawn_time {
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

        // Draw mouse cursor
        let mouse_x = self.game_state.players[0].user_input.map_mouse_x;
        let mouse_y = self.game_state.players[0].user_input.map_mouse_y;
        self.renderer.draw_mouse_cursor(mouse_x, mouse_y);

        self.renderer.flush();
    }

    pub fn set_and_draw_map(&mut self, map: Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_user_input(&mut self, player_id: usize, input: UserInput) {
        if let Some(player) = self.game_state.players.get_mut(player_id) {
            player.user_input = input;
        }
    }

    pub fn serialize_user_input(&self) -> Vec<u8> {
        // TODO serialize user input for network transmission
        Vec::new() // Placeholder
    }

    pub fn deserialize_user_input(&mut self, data: &[u8]) {
        kprintln!("Deserializing user input: {:?}", data);
        // TODO deserialize user input from bytes received over the network
    }

    pub fn serialize_state(&self) -> Vec<u8> {
        // TODO serialize game state to bytes for network transmission
        Vec::new() // Placeholder
    }

    pub fn serialize_map(&self) -> Vec<u8> {
        // TODO serialize map data to bytes for network transmission
        Vec::new() // Placeholder
    }

    pub fn deserialize_state(&mut self, data: &[u8]) {
        kprintln!("Deserializing game state: {:?}", data);
        // TODO deserialize game state from bytes received over the network
    }

    pub fn deserialize_map(&mut self, data: &[u8]) {
        kprintln!("Deserializing map data: {:?}", data);
        // TODO deserialize map data from bytes received over the network
    }
}

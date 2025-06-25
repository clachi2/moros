use crate::sys::clk::boot_time;
use crate::usr::game::renderer;
use crate::usr::game::state;
use crate::usr::game::state::{GUI_WIDTH, TICK_RATE};
use alloc::vec::Vec;

pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = state::Map::new(width, height, 12, 10, 20); // Example dimensions, adjust as needed
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
        self.game_state.players.push(state::Player {
            id: 0,
            x: pos.0 as f64,
            y: pos.1 as f64,
            alive: true,
            time_of_death: 0.0,
            user_input: state::UserInput {
                up: false,
                right: false,
                down: false,
                left: false,
                shooting: false,
                map_mouse_x: 0,
                map_mouse_y: 0,
            },
            pointing_to: (0, 0),
            color: 0x03, // Magenta
            points: 0,
            ammo: 5,
            last_shot: 0.0,
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

        // update Player positions
        for player in &mut self.game_state.players {
            if player.alive {
                // new wanted position based on movement direction and tick delta
                let new_pos = player.next_wanted_position(tick_delta);
                if  !self.game_state.map.is_pos_colliding(new_pos.0 as isize, new_pos.1 as isize) {
                    // If the new position collides with a wall, do not move
                    player.x = new_pos.0;
                    player.y = new_pos.1;
                }
            }
        }

        // update Bullet positions
        // check Collisions between Players and Bullets
        // check if dead players need to respawn
    }

    pub fn draw(&mut self) {
        // TODO draw game state, players, bullets, etc.
        self.renderer.draw_map();
        for player in &self.game_state.players {
            if player.alive {
                self.renderer
                    .draw_player(player.x as isize + GUI_WIDTH as isize, player.y as isize, player.color);
            }
        }
        // Draw mouse cursor
        let mouse_x = self.game_state.players[0].user_input.map_mouse_x; // TODO get index from somewhere else
        let mouse_y = self.game_state.players[0].user_input.map_mouse_y;
        self.renderer.draw_mouse_cursor(mouse_x, mouse_y);

        self.renderer.flush();
    }

    pub fn set_and_draw_map(&mut self, map: state::Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_user_input(&mut self, player_id: usize, input: state::UserInput) {
        if let Some(player) = self.game_state.players.get_mut(player_id) {
            player.user_input = input;
        }
    }

    pub fn try_shoot(&mut self, player_id: usize) {
        // TODO check if player can shoot and handle shooting logic
    }

    pub fn serialize_user_input(&self) -> Vec<u8> {
        // TODO serialize user input for network transmission
        Vec::new() // Placeholder
    }

    pub fn deserialize_user_input(&mut self, data: &[u8]) {
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
        // TODO deserialize game state from bytes received over the network
    }

    pub fn deserialize_map(&mut self, data: &[u8]) {
        // TODO deserialize map data from bytes received over the network
    }
}

use crate::sys::clk::epoch_time;
use crate::usr::game::renderer;
use crate::usr::game::state;
use alloc::vec::Vec;


pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let game_state = state::GameState::new();
        let renderer = renderer::Renderer::new(width, height, 8, 12, 10, 20);
        Game {
            game_state,
            renderer,
        }
    }

    pub fn init(&mut self) {
        self.renderer.init();
        self.game_state.map.set_test_map();
        self.renderer.draw_map_buffer(self.game_state.map.clone());

        // tests
        self.game_state.players.push(state::Player {
            id: 0,
            x: 5.0,
            y: 5.0,
            alive: true,
            time_of_death: 0.0,
            driving_direction: state::Direction{
                up: false,
                right: false,
                down: false,
                left: false,
            },
            pointing_to: (0, 0),
            color: 0x0f,
            points: 0,
            ammo: 5,
            last_shot: 0.0,
        });
    }

    pub fn tick(&mut self) {
        // TODO update game state, handle input, etc.

        // Tick timing
        let current_time = epoch_time();
        let tick_delta = current_time - self.game_state.last_tick;
        self.game_state.last_tick = current_time;

        // update Player positions
        for player in &self.game_state.players {
            if player.alive {
                // new wanted position based on movement direction and tick delta
                let new_pos = player.next_wanted_position(tick_delta);
                // get closest position on line from x,y to new_x,new_y without going through walls

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
                self.renderer.draw_player(
                    player.x as usize,
                    player.y as usize,
                    player.color,
                    player.driving_direction.clone(),
                );
            }
        }
        self.renderer.flush();
    }

    pub fn set_and_draw_map(&mut self, map: state::Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_player_movement(&mut self, player_id: usize, direction: state::Direction) {
        // TODO overwrite player movement direction
    }

    pub fn try_shoot(&mut self, player_id: usize) {
        // TODO check if player can shoot and handle shooting logic
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

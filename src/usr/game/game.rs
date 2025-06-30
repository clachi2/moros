use crate::api::fs;
use crate::api::fs::DeviceType::Random;
use crate::sys::clk::boot_time;
use crate::usr::game::network::{MessageType, NetworkHandler};
use crate::usr::game::renderer;
use crate::usr::game::renderer::Color;
use crate::usr::game::state;
use crate::usr::game::state::{GUI_WIDTH, Serializable, TICK_RATE, UserInput};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::hash::Hash;

pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
    network_handler: NetworkHandler,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let mut map = state::Map::new(width, height, 12, 10, 20); // Example dimensions, adjust as needed
        //map.auto_set_walls();
        let game_state = state::GameState::new(map);
        let renderer = renderer::Renderer::new(width, height, 8, 12, 10, 20);
        let network_handler = NetworkHandler::new();
        Game {
            game_state,
            renderer,
            network_handler,
        }
    }

    pub fn init(&mut self, is_server: bool) {
        self.network_handler
            .init(is_server, None)
            .expect("Failed to initialize network handler");
        if is_server {
            self.network_handler.set_server();
            self.game_state.map.auto_set_walls();
            // Set the map in network handler so it can be sent to clients
            self.network_handler.set_map(self.game_state.map.clone());
            kprintln!("Server: Map generated and ready");
        } else {
            kprintln!("Client: Waiting for map from server...");
            // Wait for map from server
            loop {
                // Client is stuck here until it receives the map
                self.network_handler
                    .send_message_type(MessageType::MapRequest, &[])
                    .expect("TODO: panic message");
                if let Some(received_map) = self.network_handler.get_received_map() {
                    self.game_state.map = received_map;
                    kprintln!("Client: Map received from server!");
                    break;
                }
                crate::sys::clk::halt(); // Small delay
            }
        }

        self.renderer.init();
        self.renderer.draw_map_buffer(self.game_state.map.clone());

        // let pos = self.game_state.map.random_pos();
        //
        // // tests
        // self.game_state.players.push(state::Player {
        //     id: 1,
        //     x: pos.0 as f64,
        //     y: pos.1 as f64,
        //     alive: true,
        //     time_of_death: 0.0,
        //     user_input: state::UserInput {
        //         up: false,
        //         right: false,
        //         down: false,
        //         left: false,
        //         shooting: false,
        //         map_mouse_x: 0,
        //         map_mouse_y: 0,
        //     },
        //     pointing_to: (0, 0),
        //     color: Color::Blue as u8,
        //     points: 0,
        //     ammo: 5,
        //     last_shot: 0.0,
        // });
    }

    pub fn deinit(&mut self) {
        self.renderer.deinit();
    }

    pub fn tick(&mut self, is_server: bool) {
        // Handle network messages
        self.handle_network_messages();

        // Tick timing
        let current_time = boot_time();
        let tick_delta = current_time - self.game_state.last_tick;
        if tick_delta < 1.0 / TICK_RATE {
            return; // Not enough time has passed for the next tick
        }
        self.game_state.last_tick = current_time;

        // update Player positions
        if is_server {
            for player in &mut self.game_state.players {
                if player.alive {
                    // new wanted position based on movement direction and tick delta
                    let new_pos = player.next_wanted_position(tick_delta);
                    if  !self.game_state.map.is_pos_colliding(new_pos.0 as isize, new_pos.1 as isize) {
                        // If the new position collides with a wall, do not move
                        player.x = new_pos.0;
                        player.y = new_pos.1;
                    }
                    else if !self.game_state.map.is_pos_colliding(player.x as isize, new_pos.1 as isize) {
                        // only move vertically if horizontal movement is blocked
                        player.y = new_pos.1;
                    }
                    else if !self.game_state.map.is_pos_colliding(new_pos.0 as isize, player.y as isize) {
                        // only move horizontally if vertical movement is blocked
                        player.x = new_pos.0;
                    }
                    else {
                        // check of each pixel of line between old and new position if it collides with a wall
                    }
                }
            }
        } else {
            // Client does not update player positions, just draws the current state
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
        //TODO
        //let mouse_x = self.game_state.players[0].user_input.map_mouse_x; // TODO get index from somewhere else
        //let mouse_y = self.game_state.players[0].user_input.map_mouse_y;
        //self.renderer.draw_mouse_cursor(mouse_x, mouse_y);

        self.renderer.flush();
    }

    pub fn set_and_draw_map(&mut self, map: state::Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_user_input(&mut self, player_id: usize, input: state::UserInput) {
        // TODO kann sein das get_mut irgendwas kpautt macht mit network aber findet id nicht ... RUST Bug idk
        if let Some(player) = self.game_state.players.get_mut(player_id) {
            player.user_input = input;
        } else {
            kprintln!("Player with ID {} not found", player_id);
        }
    }

    pub fn try_shoot(&mut self, player_id: usize) {
        // TODO check if player can shoot and handle shooting logic
    }

    pub fn serialize_user_input(&self) -> Vec<u8> {
        // Get the first player's input (for now)
        if let Some(player) = self.game_state.players.first() {
            player.user_input.serialize()
        } else {
            Vec::new()
        }
    }

    pub fn deserialize_user_input(&mut self, player_id: usize, data: &[u8]) {
        let user_input = state::UserInput::deserialize(data);
        self.set_user_input(player_id, user_input);
    }

    pub fn serialize_state(&self) -> Vec<u8> {
        self.game_state.serialize()
    }

    pub fn serialize_map(&self) -> Vec<u8> {
        self.game_state.map.serialize()
    }

    pub fn deserialize_state(&mut self, data: &[u8]) {
        self.game_state = state::GameState::deserialize(data);
        // Update the renderer with the new map
        self.renderer.draw_map_buffer(self.game_state.map.clone());
    }

    pub fn deserialize_map(&mut self, data: &[u8]) {
        let map = state::Map::deserialize(data);
        self.set_and_draw_map(map);
    }

    fn handle_network_messages(&mut self) {
        if let Ok(messages) = self.network_handler.poll_messages() {
            for (msg_type, data, sender) in messages {
                match msg_type {
                    MessageType::MapData => {
                        if !data.is_empty() {
                            let received_map = state::Map::deserialize(&data);
                            self.set_and_draw_map(received_map);
                            //kprintln!("Client: Map updated from server");
                        }
                    }
                    MessageType::PlayerInput => {
                        // Extract player ID from sender's IP
                        let player_id = match sender.endpoint.addr {
                            smoltcp::wire::IpAddress::Ipv4(ipv4) => ipv4.octets()[3] as usize,
                            _ => 0,
                        };

                        if !data.is_empty() {
                            //TODO PROBLEM MIT ID REF
                            self.deserialize_user_input(0, &data);
                            //kprintln!("Received input from player {}", player_id);
                        }
                    }
                    MessageType::GameStateUpdate => {
                        // Client receives game state update from server
                        if !data.is_empty() {
                            self.deserialize_state(&data);
                            //kprintln!("Client: Game state updated from server");
                        }
                    }
                    MessageType::Connect => {
                        kprintln!("New client connected: {:?}", sender);
                        //TODO: neuen spieler erstellen und zum Spiel hinzufügen
                        let pos = self.game_state.map.random_pos();
                        let player_id = match sender.endpoint.addr {
                            smoltcp::wire::IpAddress::Ipv4(ipv4) => {
                                let octets = ipv4.octets()[3];
                                octets
                            }

                            _ => 0,
                        };
                        kprintln!("Adding player with ID {} at position {:?}", player_id, pos);
                        for player in &self.game_state.players {
                            if player.id == player_id as usize {
                                kprintln!(
                                    "Player with ID {} already exists, not adding again",
                                    player_id
                                );
                                return; // Player already exists, do not add again
                            }
                        }

                        let new_player = state::Player {
                            //id: player_id as usize,
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
                            color: Color::Green as u8, // TODO: Randomize color
                            points: 0,
                            ammo: 5,
                            last_shot: 0.0,
                        };
                        self.game_state.players.push(new_player);
                    }
                    _ => {
                        // Handle other message types as needed
                        //kprintln!("Received message of type {:?} from {:?}", msg_type, sender);
                    }
                }
            }
        }
    }

    pub fn send_user_input_to_server(&mut self, player_id: usize) -> Result<(), String> {
        for player in &self.game_state.players {
            if player.id == player_id {
                let input_data = player.user_input.serialize();
                self.network_handler.send_player_input(&input_data).expect("TODO: panic message");
                return Ok(());
            }
        }
        Err("Player not found".to_string())

        // if let Some(player) = self.game_state.players.get_mut(player_id) {
        //     let input_data = player.user_input.serialize();
        //     self.network_handler.send_player_input(&input_data)
        // } else {
        //     Err("Player not found".to_string())
        // }
    }

    pub fn broadcast_game_state_to_clients(&mut self) -> Result<(), String> {
        let state_data = self.serialize_state();
        self.network_handler.broadcast_game_state(&state_data)
    }

    pub fn get_network_handler(&mut self) -> &mut NetworkHandler {
        &mut self.network_handler
    }
}

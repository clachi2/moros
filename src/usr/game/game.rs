use crate::sys::clk::boot_time;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::network::{MessageType, NetworkHandler};
use crate::usr::game::player::{Player, UserInput};
use crate::usr::game::renderer;
use crate::usr::game::renderer::Color;
use crate::usr::game::state;
use crate::usr::game::state::{
    GUI_WIDTH, PLAYER_SIZE, SHOOTING_RATE_PER_SECOND, Serializable, TICK_RATE,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub(crate) struct Game {
    game_state: state::GameState,
    renderer: renderer::Renderer,
    network_handler: NetworkHandler,
}

impl Game {
    pub fn new(width: usize, height: usize) -> Self {
        let map = Map::new(width, height, 12, 10, 20); // Example dimensions, adjust as needed
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
            // Set the map in network handler, so it can be sent to clients
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
                }

                if player.alive && player.user_input.shooting && player.ammo > 0 {
                    // Check if enough time has passed since last shot (rate limiting)
                    let time_since_last_shot = current_time - player.last_shot;
                    let min_shot_interval = 1.0 / SHOOTING_RATE_PER_SECOND;

                    if time_since_last_shot >= min_shot_interval {
                        // Calculate target position from mouse coordinates
                        let target_x = (player.user_input.map_mouse_x - GUI_WIDTH as isize) as f64;
                        let target_y = player.user_input.map_mouse_y as f64;

                        // Spawn bullet
                        // self.game_state.spawn_bullet(player.id, target_x, target_y);
                        if player.ammo > 0 {
                            let bullet = Bullet::new(
                                player.x + (PLAYER_SIZE as f64 / 2.0), // Center of player
                                player.y + (PLAYER_SIZE as f64 / 2.0),
                                target_x,
                                target_y,
                                &self.game_state.map,
                            );
                            self.game_state.bullets.push(bullet);
                        }

                        // Update player state
                        player.ammo -= 1;
                        player.last_shot = current_time;
                    }
                }
            }
        } else {
            // Client does not update player positions, just draws the current state
        }

        // Update Bullet positions
        self.game_state
            .bullets
            .retain_mut(|bullet| bullet.update(tick_delta));

        // Check Collisions between Players and Bullets
        // self.check_player_bullet_collisions();

        // Check if dead players need to respawn
        self.handle_player_respawning(current_time);
    }

    fn check_player_bullet_collisions(&mut self) {
        let mut bullets_to_remove = Vec::new();
        let mut players_to_kill = Vec::new();

        for (bullet_idx, bullet) in self.game_state.bullets.iter().enumerate() {
            for (player_idx, player) in self.game_state.players.iter().enumerate() {
                if player.alive && self.bullet_hits_player(bullet, player) {
                    bullets_to_remove.push(bullet_idx);
                    players_to_kill.push(player_idx);
                    break; // One bullet can only hit one player
                }
            }
        }

        // Remove bullets that hit players (in reverse order to maintain indices)
        for &bullet_idx in bullets_to_remove.iter().rev() {
            self.game_state.bullets.remove(bullet_idx);
        }

        // Kill players that were hit
        let current_time = boot_time();
        for &player_idx in &players_to_kill {
            if let Some(player) = self.game_state.players.get_mut(player_idx) {
                player.alive = false;
                player.time_of_death = current_time;
                // TODO: Award points to the shooter
            }
        }
    }

    fn bullet_hits_player(&self, bullet: &Bullet, player: &Player) -> bool {
        let bullet_size = 2.0; // Small bullet size
        let player_size = PLAYER_SIZE as f64;

        // Simple AABB collision detection
        bullet.x < player.x + player_size
            && bullet.x + bullet_size > player.x
            && bullet.y < player.y + player_size
            && bullet.y + bullet_size > player.y
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
        // TODO draw game state, players, bullets, etc.
        // Draw map
        self.renderer.draw_map();

        // Draw players
        for player in &self.game_state.players {
            if player.alive {
                self.renderer.draw_player(
                    player.x as isize + GUI_WIDTH as isize,
                    player.y as isize,
                    player.color,
                );
            }
        }

        // Draw bullets
        for bullet in &self.game_state.bullets {
            self.renderer
                .draw_bullet(bullet.x as isize + GUI_WIDTH as isize, bullet.y as isize);
        }

        // Draw mouse cursor
        // TODO maus muss irgendwie anders gerendert werden
        if !self.game_state.players.is_empty() {
            let player = &self.game_state.players[0];
            let mouse_x = player.user_input.map_mouse_x;
            let mouse_y = player.user_input.map_mouse_y;
            self.renderer.draw_mouse_cursor(mouse_x, mouse_y);
        }
        //let mouse_x = self.game_state.players[0].user_input.map_mouse_x;
        //let mouse_y = self.game_state.players[0].user_input.map_mouse_y;
        //self.renderer.draw_mouse_cursor(mouse_x, mouse_y);

        self.renderer.flush();
    }

    pub fn set_and_draw_map(&mut self, map: Map) {
        self.game_state.map = map.clone();
        self.renderer.draw_map_buffer(map);
    }

    pub fn set_user_input(&mut self, player_id: usize, input: UserInput) {
        if let Some(player) = self.game_state.players.get_mut(player_id) {
            player.user_input = input;
        } else {
            kprintln!("Player with ID {} not found", player_id);
        }
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
        self.game_state = state::GameState::deserialize(data);
        // Update the renderer with the new map
        self.renderer.draw_map_buffer(self.game_state.map.clone());
    }

    pub fn deserialize_map(&mut self, data: &[u8]) {
        let map = Map::deserialize(data);
        self.set_and_draw_map(map);
    }

    fn handle_network_messages(&mut self) {
        if let Ok(messages) = self.network_handler.poll_messages() {
            for (msg_type, data, sender) in messages {
                match msg_type {
                    MessageType::MapData => {
                        if !data.is_empty() {
                            let received_map = Map::deserialize(&data);
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
                        //TODO neuen spieler erstellen und zum Spiel hinzufügen
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

                        let new_player = Player {
                            //id: player_id as usize,
                            id: 0,
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
                            color: Color::Green as u8, // TODO: Randomize color
                            points: 0,
                            ammo: 1000,
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
                self.network_handler
                    .send_player_input(&input_data)
                    .expect("TODO: panic message");
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
        kprintln!(
            "Game state size: {} bytes, {} players, {} bullets",
            state_data.len(),
            self.game_state.players.len(),
            self.game_state.bullets.len()
        );
        self.network_handler.broadcast_game_state(&state_data)
    }

    pub fn get_network_handler(&mut self) -> &mut NetworkHandler {
        &mut self.network_handler
    }
}

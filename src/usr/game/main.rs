use crate::api::console::Style;
use crate::api::process::ExitCode;
use crate::kprintln;
use crate::sys;
use crate::sys::clk::boot_time;
use crate::sys::console;
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::network::{MessageType, NetworkHandler};
use crate::usr::game::tanks::game::map_demo;
// use crate::usr::game::tanks::map::Map;
use alloc::format;
use alloc::string::String;

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SERVER_IP: &str = "192.168.0.1";
const SERVER_PORT: u16 = 1234;
const BROADCAST_RATE: f64 = 64.0; // Broadcast rate per second

pub static CLIENT_SEND_DELAY: f64 = 0.0; // 100ms delay
pub static CLIENT_RECEIVE_DELAY: f64 = 0.0; // 50ms delay
pub static SERVER_SEND_DELAY: f64 = 0.0; // 80ms delay
pub static SERVER_RECEIVE_DELAY: f64 = 0.0; // 30ms delay
pub static DROP_PROBABILITY: f64 = 0.0; // 10% packet loss

static mut RNG_STATE: u64 = 12345;

/// Generates a pseudo-random floating-point number between 0.0 and 1.0.
fn simple_random() -> f64 {
    unsafe {
        RNG_STATE = RNG_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        (RNG_STATE as f64) / (u64::MAX as f64)
    }
}

/// Main entry point for the game application.
///
/// This function parses command-line arguments to determine the execution mode and
/// starts either a server instance, a client instance, or displays help information.
/// It supports server mode, client mode with configurable IP addresses, and a map
/// demonstration mode.
///
/// # Parameters
/// - `args`: A slice of string slices containing the command-line arguments
///
/// # Returns
/// - `Ok(())` if the application executed successfully
/// - `Err(ExitCode)` if an error occurred or invalid arguments were provided
///
/// # Supported Arguments
/// - `-h, --help`: Display help information
/// - `-s, --server`: Start in server mode
/// - `-ip=<digit>`: Start as client with IP 192.168.0.<digit> (digit must be 2-255)
/// - `-m, --mapdemo`: Start the map generation demonstration
pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.iter().any(|&arg| arg == "-h" || arg == "--help") {
        return help();
    }
    if args.iter().any(|&arg| arg == "-m" || arg == "--mapdemo") {
        return map_demo(WIDTH, HEIGHT);
    }
    let is_server = args.iter().any(|&arg| arg == "-s" || arg == "--server");

    // Parse IP argument
    let client_ip_digit = if let Some(ip_arg) = args.iter().find(|&&arg| arg.starts_with("-ip=")) {
        if let Some(digit_str) = ip_arg.strip_prefix("-ip=") {
            match digit_str.parse::<u8>() {
                Ok(digit) if digit >= 2 && digit <= 255 => Some(digit),
                _ => {
                    kprintln!("Invalid IP digit. Must be between 2 and 255.");
                    return Err(ExitCode::Failure);
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    if is_server {
        kprintln!("Starting server at {}:{}", SERVER_IP, SERVER_PORT);
        return server();
    } else if let Some(ip_digit) = client_ip_digit {
        kprintln!("Starting client with IP 192.168.0.{}", ip_digit);
        return client(Some(ip_digit));
    } else {
        kprintln!("No valid arguments provided. Use -s for server or -ip=<digit> for client.");
        return Err(ExitCode::Failure);
    }
}

/// Starts the game client with the specified IP configuration.
///
/// This function initializes a game client that connects to a server, receives
/// game state updates, sends user input, and renders the game. The client handles
/// network communication, user input processing, and game state synchronization
/// with the server.
///
/// # Parameters
/// - `ip_digit`: An optional byte representing the last octet of the client's IP address.
///   If provided, the client will use IP 192.168.0.<ip_digit>. If None, a default IP is used.
///
/// # Returns
/// - `Ok(())` if the client shut down cleanly
/// - `Err(ExitCode)` if an error occurred during client execution
///
/// # Game Loop
/// The client runs in a continuous loop that:
/// 1. Requests and receives map data from the server
/// 2. Handles incoming network messages (game state updates, etc.)
/// 3. Processes user input (keyboard and mouse)
/// 4. Sends input data to the server
/// 5. Updates and renders the game state
pub fn client(ip_digit: Option<u8>) -> Result<(), ExitCode> {
    let mut game = crate::usr::game::tanks::game::Game::new(WIDTH, HEIGHT);
    let mut network_handler = NetworkHandler::new();

    let client_ip = if let Some(digit) = ip_digit {
        Some(format!("192.168.0.{}", digit))
    } else {
        None
    };

    // Initialize network handler
    network_handler
        .init(false, client_ip.as_deref())
        .expect("Initializing network failed");

    // Initialize game
    game.init();

    //init a player for the client if he plays "locally"
    //should be deleted if the server sends the player data
    game.add_player(ip_digit.unwrap() as usize);

    // add a dummy player for testing
    game.add_player(10);

    let mut set_map_from_server = false;
    let mut set_player_index = false;

    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let mut shooting = false;
    get_mouse_buffer().clear_events();

    game.tick();
    loop {
        // Check if the map has been set from the server
        if !set_map_from_server {
            network_handler
                .send_message_type(MessageType::MapRequest, &[])
                .expect("TODO: panic message");
            let received_map = network_handler.get_received_map();
            if let Some(map) = received_map {
                game.deserialize_map(&map);
                set_map_from_server = true;
                kprintln!("Client: Map received from server!");
            }
        }

        if !set_player_index {
            game.set_current_player(ip_digit.unwrap() as usize);
        }

        // Handle network messages
        if let Err(e) = handle_network_messages(&mut game, &mut network_handler) {
            kprintln!("Failed to handle network messages: {}", e);
        }

        //handle input
        // let user_input = get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
        // let user_input =
        //     UserInput::get_next_input(&mut mouse_x, &mut mouse_y, &mut shooting, WIDTH, HEIGHT);
        // game.set_user_input(ip_digit.unwrap() as usize, user_input);
        game.update_user_input(ip_digit.unwrap() as usize, WIDTH, HEIGHT);

        // Send user input to server
        if let Err(e) =
            send_user_input_to_server(&game, &mut network_handler, ip_digit.unwrap() as usize)
        {
            kprintln!("Failed to send user input to server: {}", e);
        }

        console::disable_echo();
        console::enable_raw();
        {
            let mut stdin = console::STDIN.lock();
            if !stdin.is_empty() {
                match stdin.remove(0) {
                    'q' => {
                        //send disconnect message to server
                        for _ in 0..50 {
                            // send multiple disconnect messages to ensure server receives it
                            network_handler
                                .send_message_type(MessageType::Disconnect, &[])
                                .expect("Failed to send disconnect message");
                        }

                        game.deinit();
                        return Ok(());
                    }
                    'y' => {
                        // TODO remove this, just for testing
                        game.set_new_random_map(WIDTH, HEIGHT);
                    }
                    _ => {}
                }
            }
        }
        console::enable_echo();
        console::disable_raw();

        game.tick();
        game.draw();

        sys::clk::halt();
    }
}

/// Starts the game server that manages multiplayer game sessions.
///
/// This function initializes a game server that accepts client connections,
/// processes player input from multiple clients, updates the authoritative
/// game state, and broadcasts state updates to all connected clients.
/// The server operates as the central authority for game logic and state.
///
/// # Returns
/// - `Ok(())` if the server shut down cleanly
/// - `Err(ExitCode)` if an error occurred during server execution
///
/// # Server Responsibilities
/// The server handles:
/// - Client connection management (connect/disconnect)
/// - Processing player input from all clients
/// - Authoritative game state updates (physics, collisions, etc.)
/// - Broadcasting game state to all clients at a fixed rate
/// - Map data distribution to new clients
pub fn server() -> Result<(), ExitCode> {
    let mut game = crate::usr::game::tanks::game::Game::new(WIDTH, HEIGHT);
    let mut network_handler = NetworkHandler::new();

    // Initialize network handler
    network_handler
        .init(true, None)
        .expect("Initializing network failed");
    network_handler.set_server();

    // Initialize game
    game.init();

    //network_handler needs map
    network_handler.set_map(game.serialize_map());

    let mut last_broadcast = 0.0;

    loop {
        // Handle network messages
        if let Err(e) = handle_network_messages(&mut game, &mut network_handler) {
            kprintln!("Failed to handle network messages: {}", e);
        }

        console::disable_echo();
        console::enable_raw();
        {
            let mut stdin = console::STDIN.lock();
            if !stdin.is_empty() {
                match stdin.remove(0) {
                    'q' => {
                        game.deinit();
                        return Ok(());
                    }
                    'y' => {
                        game.set_new_random_map(WIDTH, HEIGHT);
                    }
                    _ => {}
                }
            }
        }
        console::enable_echo();
        console::disable_raw();

        game.tick();
        game.draw();

        // Broadcast game state to all clients (rate limited)
        let current_time = boot_time();
        if current_time - last_broadcast >= 1.0 / BROADCAST_RATE {
            if let Err(e) = broadcast_game_state_to_clients(&game, &mut network_handler) {
                kprintln!("Failed to broadcast game state: {}", e);
            }
            last_broadcast = current_time;
        }

        sys::clk::halt();
    }
}

/// Processes all incoming network messages and updates the game state accordingly.
///
/// This function polls for incoming network messages and dispatches them based on
/// their message type. It handles different types of messages including map data,
/// player input, game state updates, and client connection/disconnection events.
/// The function also simulates network conditions such as packet loss for testing.
///
/// # Parameters
/// - `game`: A mutable reference to the game instance that will be updated based on received messages
/// - `network_handler`: A mutable reference to the network handler used for message polling
///
/// # Returns
/// - `Ok(())` if all messages were processed successfully
/// - `Err(String)` if an error occurred during message processing
///
/// # Message Types Handled
/// - `MapData`: Updates the game map from server data
/// - `PlayerInput`: Processes input from remote players (server-side)
/// - `GameStateUpdate`: Updates local game state from server (client-side)
/// - `Connect`: Handles new client connections (server-side)
/// - `Disconnect`: Handles client disconnections (server-side)
fn handle_network_messages(
    game: &mut crate::usr::game::tanks::game::Game,
    network_handler: &mut NetworkHandler,
) -> Result<(), String> {
    if let Ok(messages) = network_handler.poll_messages() {
        for (msg_type, data, sender) in messages {
            match msg_type {
                MessageType::MapData => {
                    if !data.is_empty() {
                        game.deserialize_map(&data);
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
                        //simulate packet loss
                        if simple_random() < DROP_PROBABILITY {
                            kprintln!("Simulated packet loss for players input {}", player_id);
                            continue; // Simulate packet loss
                        }

                        game.deserialize_user_input(player_id, &data);
                    }
                }
                MessageType::GameStateUpdate => {
                    //simulate packet loss
                    if simple_random() < DROP_PROBABILITY {
                        kprintln!("Simulated packet loss for game state update");
                        continue; // Simulate packet loss
                    }

                    // Client receives game state update from server
                    if !data.is_empty() {
                        game.deserialize_state(&data);
                    }
                }
                MessageType::Connect => {
                    kprintln!("New client connected: {:?}", sender);
                    handle_new_client_connection(game, sender)?;
                }
                MessageType::Disconnect => {
                    // Extract player ID from sender's IP
                    let player_id = match sender.endpoint.addr {
                        smoltcp::wire::IpAddress::Ipv4(ipv4) => ipv4.octets()[3] as usize,
                        _ => 0,
                    };
                    kprintln!("Client disconnected: {}", player_id);
                    handle_disconnected_client(game, player_id)?;
                }
                _ => {
                    // Handle other message types as needed
                    kprintln!("Received message of type {:?} from {:?}", msg_type, sender);
                }
            }
        }
    }
    Ok(())
}

/// Handles the connection of a new client to the server.
///
/// # Parameters
/// - `game`: A mutable reference to the game instance where the new player will be added
/// - `sender`: Network metadata containing the client's connection information
///
/// # Returns
/// - `Ok(())` if the client was successfully added to the game
/// - `Err(String)` if an error occurred during client addition
///
/// # Player ID Assignment
/// The player ID is determined by the last octet of the client's IPv4 address.
/// For example, a client with IP 192.168.0.5 will have player ID 5.
fn handle_new_client_connection(
    game: &mut crate::usr::game::tanks::game::Game,
    sender: smoltcp::socket::udp::UdpMetadata,
) -> Result<(), String> {
    let player_id = match sender.endpoint.addr {
        smoltcp::wire::IpAddress::Ipv4(ipv4) => {
            let octets = ipv4.octets()[3];
            octets
        }
        _ => 0,
    };

    kprintln!("Adding player with ID {} at random position", player_id);

    // Add player to game
    let player_id = player_id as usize;
    game.add_player(player_id);
    Ok(())
}

/// Handles the disconnection of a client from the server.
fn handle_disconnected_client(
    game: &mut crate::usr::game::tanks::game::Game,
    player_id: usize,
) -> Result<(), String> {
    game.remove_player(player_id);
    Ok(())
}

/// Sends the local player's input data to the server.
///
/// This function serializes the specified player's current input state (movement,
/// shooting, mouse position) and transmits it to the server for processing.
/// It includes packet loss simulation for testing network resilience.
///
/// # Parameters
/// - `game`: A reference to the game instance containing the player's input data
/// - `network_handler`: A mutable reference to the network handler for message transmission
/// - `player_id`: The unique identifier of the player whose input should be sent
///
/// # Returns
/// - `Ok(())` if the input data was sent successfully (or simulated as lost)
/// - `Err(String)` if an error occurred during input transmission
///
/// # Network Simulation
/// This function may simulate packet loss based on the global `DROP_PROBABILITY`
/// constant, dropping the packet without sending it to test network resilience.
fn send_user_input_to_server(
    game: &crate::usr::game::tanks::game::Game,
    network_handler: &mut NetworkHandler,
    player_id: usize,
) -> Result<(), String> {
    //Simulate packet loss
    if simple_random() < DROP_PROBABILITY {
        kprintln!("Simulated packet loss for player {}", player_id);
        return Ok(()); // Simulate packet loss
    }

    let input_data = game.serialize_user_input(player_id);
    network_handler
        .send_player_input(&input_data)
        .map_err(|e| format!("Failed to send user input: {}", e))
}

/// Broadcasts the current game state to all connected clients.
///
/// This function serializes the complete game state (including all players, bullets,
/// and other game objects) and sends it to all connected clients.
/// 
/// # Parameters
/// - `game`: A reference to the game instance containing the current authoritative state
/// - `network_handler`: A mutable reference to the network handler for broadcasting
///
/// # Returns
/// - `Ok(())` if the game state was broadcast successfully (or simulated as lost)
/// - `Err(String)` if an error occurred during state transmission
///
/// # Network Simulation
/// This function may simulate packet loss based on the global `DROP_PROBABILITY`
/// constant, dropping the packet without sending it to test network resilience.
///
/// # Usage
/// This function is typically called by the server at regular intervals (defined by
/// `BROADCAST_RATE`) to keep all clients synchronized with the authoritative game state.
fn broadcast_game_state_to_clients(
    game: &crate::usr::game::tanks::game::Game,
    network_handler: &mut NetworkHandler,
) -> Result<(), String> {
    // Simulate packet loss
    if simple_random() < DROP_PROBABILITY {
        kprintln!("Simulated packet loss for game state broadcast");
        return Ok(()); // Simulate packet loss
    }

    let state_data = game.serialize_state();
    network_handler
        .broadcast_game_state(&state_data)
        .map_err(|e| format!("Failed to broadcast game state: {}", e))
}

/// Displays help information for the game application.
pub fn help() -> Result<(), ExitCode> {
    let csi_option = Style::color("aqua");
    let csi_title = Style::color("yellow");
    let csi_reset = Style::reset();
    println!(
        "{}Usage:{} game {}-s|--server -ip=<digit>{}",
        csi_title, csi_reset, csi_option, csi_reset
    );
    println!("{}Options:{}", csi_title, csi_reset);
    println!(
        "  {}-s, --server{}       Start as server (IP: 192.168.0.1)",
        csi_option, csi_reset
    );
    println!(
        "  {}-ip=<digit>{}        Set client IP to 192.168.0.<digit> (2-255)",
        csi_option, csi_reset
    );
    println!(
        "  {}-m, --mapdemo{}      Starts the map demo",
        csi_option, csi_reset
    );
    println!("{}Examples:{}", csi_title, csi_reset);
    println!("  game -s            Start server");
    println!("  game -ip=2         Connect as client with IP 192.168.0.2");
    println!("  game -ip=3         Connect as client with IP 192.168.0.3");
    Ok(())
}

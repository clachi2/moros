use crate::api::console::Style;
use crate::api::process::ExitCode;
use crate::kprintln;
use crate::sys;
use crate::sys::clk::boot_time;
use crate::sys::console;
use crate::sys::keyboard::{DOWN, LEFT, RIGHT, UP};
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::map::Map;
use crate::usr::game::network::{MessageType, NetworkHandler};
use crate::usr::game::player::UserInput;
use crate::usr::game::state::{GUI_WIDTH, Serializable, WALL_DENSITY};
use alloc::format;
use alloc::string::String;
use crate::usr::game::renderer::Color;

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SERVER_IP: &str = "192.168.0.1";
const SERVER_PORT: u16 = 1234;
const BROADCAST_RATE: f64 = 64.0; // Broadcast rate per second

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.iter().any(|&arg| arg == "-h" || arg == "--help") {
        return help();
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

pub fn client(ip_digit: Option<u8>) -> Result<(), ExitCode> {
    let mut game = crate::usr::game::game::Game::new(WIDTH, HEIGHT);
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
    game.add_player(100);

    let mut set_map_from_server = false;
    let mut set_player_index = false;

    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let mut shooting = false;
    get_mouse_buffer().clear_events();

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
        let user_input = get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
        game.set_user_input(ip_digit.unwrap() as usize, user_input);

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
                        game.deinit();
                        return Ok(());
                    }
                    'y' => {
                        // TODO remove this, just for testing
                        let mut map = Map::new(320, 200, 12, 10, 20);
                        map.auto_set_walls();
                        game.set_and_draw_map(map);
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

pub fn server() -> Result<(), ExitCode> {
    let mut game = crate::usr::game::game::Game::new(WIDTH, HEIGHT);
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
                        // TODO remove this, just for testing
                        let mut map = Map::new(320, 200, 12, 10, 20);
                        map.auto_set_walls();
                        game.set_and_draw_map(map);
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

fn map_demo() {
    let mut map = Map::new(320, 200, 12, 10, 20);
    let mut renderer = renderer::Renderer::new(WIDTH, HEIGHT, 8, 12, 10, 20);
    renderer.init();
    renderer.clear();

    // draw all colors
    // renderer.draw_all_colors();
    // renderer.flush();
    // wait_until_pressend_and_released();
    // renderer.clear();

    // generate random walls
    map.clear_walls();
    map.add_random_walls(WALL_DENSITY);
    map.set_outer_walls();
    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
    renderer.flush();
    wait_until_pressend_and_released();

    let mut areas = map.find_distinct_areas();
    // areas.sort_by(|a, b| a.len().cmp(&b.len())); // Sort areas by size (smallest first)

    while areas.len() > 1 {
        renderer.draw_map_buffer(map.clone());
        renderer.draw_map();
        renderer.draw_areas(&areas);
        renderer.flush();
        wait_until_pressend_and_released();
        for area in areas.iter() {
            for tile_coord in area {
                let x = tile_coord.0;
                let y = tile_coord.1;
                let index = y * map.tiles_x + x;
                let index_right = y * map.tiles_x + (x + 1);
                let index_down = (y + 1) * map.tiles_x + x;
                if y > 0 && map.tiles[index].up && !area.contains(&(tile_coord.0, tile_coord.1 - 1)) {
                    map.tiles[index].up = false;
                    let line = map.wall_coords(x as isize, y as isize, 0);
                    renderer.framebuffer
                        .draw_line(line.0 + GUI_WIDTH as isize, line.1, line.2 + GUI_WIDTH as isize, line.3, Color::Red as u8);
                    break;
                }
                if x < map.tiles_x - 1 && (map.tiles[index].right || map.tiles[index_right].left) && !area.contains(&(tile_coord.0 + 1, tile_coord.1)) {
                    map.tiles[index].right = false;
                    map.tiles[index_right].left = false;
                    let line = map.wall_coords(x as isize, y as isize, 1);
                    renderer.framebuffer
                        .draw_line(line.0 + GUI_WIDTH as isize + 1, line.1, line.2 + GUI_WIDTH as isize + 1, line.3, Color::Red as u8);
                    break;
                }
                if y < map.tiles_y - 1 && (map.tiles[index].down || map.tiles[index_down].up) && !area.contains(&(tile_coord.0, tile_coord.1 + 1)) {
                    map.tiles[index].down = false;
                    map.tiles[index_down].up = false;
                    let line = map.wall_coords(x as isize, y as isize, 2);
                    renderer.framebuffer
                        .draw_line(line.0 + GUI_WIDTH as isize, line.1 + 1, line.2 + GUI_WIDTH as isize, line.3 + 1, Color::Red as u8);
                    break;
                }
                if x > 0 && map.tiles[index].left && !area.contains(&(tile_coord.0 - 1, tile_coord.1)) {
                    map.tiles[index].left = false;
                    let line = map.wall_coords(x as isize, y as isize, 3);
                    renderer.framebuffer
                        .draw_line(line.0 + GUI_WIDTH as isize, line.1, line.2 + GUI_WIDTH as isize, line.3, Color::Red as u8);
                    break;
                }
            }
        }
        renderer.flush();
        wait_until_pressend_and_released();
        areas = map.find_distinct_areas();
    }

    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
    renderer.draw_areas(&areas);
    renderer.flush();
    wait_until_pressend_and_released();


    renderer.deinit();
}

fn wait_until_pressend_and_released() {
    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let mut shooting = false;
    get_mouse_buffer().clear_events();
    while !shooting {
        get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
    }
    while shooting {
        get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
    }
}

fn get_user_input(mouse_x: &mut i32, mouse_y: &mut i32, shooting: &mut bool) -> UserInput {
    // get mouse and keyboard input
    let ord = core::sync::atomic::Ordering::Relaxed;
    let up = UP.load(ord);
    let down = DOWN.load(ord);
    let left = LEFT.load(ord);
    let right = RIGHT.load(ord);
    let mut new_shooting = false;
    let mut atleast_once = false;
    while let Some(event) = get_mouse_buffer().get_last_event() {
        atleast_once = true;
        *mouse_x += event.x_movement as i32;
        *mouse_y += event.y_movement as i32;
        if *mouse_x < 0 {
            *mouse_x = 0;
        } else if *mouse_x >= WIDTH as i32 {
            *mouse_x = (WIDTH - 1) as i32;
        }
        if *mouse_y < 0 {
            *mouse_y = 0;
        } else if *mouse_y >= HEIGHT as i32 {
            *mouse_y = (HEIGHT - 1) as i32;
        }
        new_shooting = new_shooting | event.is_left_click(); // shooting if left mouse button is pressed atleast once per tick
    }
    let map_mouse_x = *mouse_x as isize;
    let map_mouse_y = *mouse_y as isize;

    if atleast_once {
        *shooting = new_shooting; // update shooting state only if there was a mouse event
    }
    let shooting = if atleast_once {
        new_shooting
    } else {
        *shooting // keep the previous state if no mouse event occurred
    };

    let user_input = UserInput {
        up,
        down,
        left,
        right,
        shooting,
        map_mouse_x,
        map_mouse_y,
    };
    user_input
}

fn handle_network_messages(
    game: &mut crate::usr::game::game::Game,
    network_handler: &mut NetworkHandler,
) -> Result<(), String> {
    if let Ok(messages) = network_handler.poll_messages() {
        for (msg_type, data, sender) in messages {
            match msg_type {
                MessageType::MapData => {
                    if !data.is_empty() {
                        let received_map = Map::deserialize(&data);
                        game.set_and_draw_map(received_map);
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
                        game.deserialize_user_input(player_id, &data);
                        //kprintln!("Received input from player {}", player_id);
                    }
                }
                MessageType::GameStateUpdate => {
                    // Client receives game state update from server
                    if !data.is_empty() {
                        game.deserialize_state(&data);
                        //kprintln!("Client: Game state updated from server");
                    }
                }
                MessageType::Connect => {
                    kprintln!("New client connected: {:?}", sender);
                    handle_new_client_connection(game, sender)?;
                }
                _ => {
                    // Handle other message types as needed
                    //kprintln!("Received message of type {:?} from {:?}", msg_type, sender);
                }
            }
        }
    }
    Ok(())
}

fn handle_new_client_connection(
    game: &mut crate::usr::game::game::Game,
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

fn send_user_input_to_server(
    game: &crate::usr::game::game::Game,
    network_handler: &mut NetworkHandler,
    player_id: usize,
) -> Result<(), String> {
    let input_data = game.serialize_user_input(player_id);
    network_handler
        .send_player_input(&input_data)
        .map_err(|e| format!("Failed to send user input: {}", e))
}

fn broadcast_game_state_to_clients(
    game: &crate::usr::game::game::Game,
    network_handler: &mut NetworkHandler,
) -> Result<(), String> {
    let state_data = game.serialize_state();
    network_handler
        .broadcast_game_state(&state_data)
        .map_err(|e| format!("Failed to broadcast game state: {}", e))
}

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
    println!("{}Examples:{}", csi_title, csi_reset);
    println!("  game -s                Start server");
    println!("  game -ip=2             Connect as client with IP 192.168.0.2");
    println!("  game -ip=3             Connect as client with IP 192.168.0.3");
    Ok(())
}

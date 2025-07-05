use alloc::format;
use nom::AsChar;
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

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SERVER_IP: &str = "192.168.0.1";
const SERVER_PORT: u16 = 1234;
const BROADCAST_RATE: f64 = 10.0; // Broadcast rate per second

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

    Ok(())


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
    network_handler.init(false, client_ip.as_deref())
        .expect("Initializing network failed");

    // Initialize game
    game.init();

    //init a player for the client if he plays "localy"
    //should be deleted if the server sends the player data
    game.add_player(ip_digit.unwrap() as usize);

    let mut map_from_server_set = false;

    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let mut shooting = false;
    get_mouse_buffer().clear_events();

    loop {

        // Check if the map has been set from the server
        if !map_from_server_set {
            network_handler
                .send_message_type(MessageType::MapRequest, &[])
                .expect("TODO: panic message");
            let received_map = network_handler.get_received_map();
            if let Some(map) = received_map {
                game.deserialize_map(&map);
                map_from_server_set = true;
                kprintln!("Client: Map received from server!");
            }
        }

        //handle input
        let user_input = get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
        game.set_user_input(ip_digit.unwrap() as usize, user_input);

        // if let Err(e) = game.send_user_input_to_server(client_ip_digit.unwrap() as usize) {
        //     kprintln!("Failed to send user input to server: {}", e);
        // }

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

        // parse input to game
        let user_input = get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
        game.set_user_input(ip_digit.unwrap() as usize, user_input);

        game.tick();
        game.draw();

        sys::clk::halt();
    }


}

pub fn server() -> Result<(), ExitCode> {

    let mut game = crate::usr::game::game::Game::new(WIDTH, HEIGHT);
    let mut network_handler = NetworkHandler::new();

    // Initialize network handler
    network_handler.init(true, None)
        .expect("Initializing network failed");
    network_handler.set_server();

    // Initialize network handler
    game.init();

    //network_handler needs map
    network_handler.set_map(game.serialize_map());

    let mut last_broadcast = 0.0;

    loop {

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
        // let current_time = boot_time();
        // if current_time - last_broadcast >= 1.0 / BROADCAST_RATE {
        //     if let Err(e) = game.broadcast_game_state_to_clients() {
        //         kprintln!("Failed to broadcast game state: {}", e);
        //     }
        //     last_broadcast = current_time;
        // }

        sys::clk::halt();
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

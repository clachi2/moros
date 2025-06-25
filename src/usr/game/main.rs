use crate::api::console::Style;
use crate::api::process::ExitCode;
use crate::sys;
use crate::sys::console;
use crate::sys::keyboard::{DOWN, LEFT, RIGHT, UP};
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::state::{Map, UserInput};

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SERVER_IP: &str = "192.168.0.1";
const SERVER_PORT: u16 = 1234;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.iter().any(|&arg| arg == "-h" || arg == "--help") {
        return help();
    }
    let is_server = args.iter().any(|&arg| arg == "-s" || arg == "--server");
    let ip = args.iter().find(|&&arg| arg.starts_with("-ip="));

    let mut game = crate::usr::game::game::Game::new(WIDTH, HEIGHT);
    game.init();
    kprintln!("Starting game...");

    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    get_mouse_buffer().clear_events();

    loop {
        // parse input to game
        let user_input = get_user_input(&mut mouse_x, &mut mouse_y);
        game.set_user_input(0, user_input);

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
                    'y' => { // TODO remove this, just for testing
                        let mut map = Map::new(320, 200, 12, 10, 20);
                        map.auto_set_walls();
                        game.set_and_draw_map(map);
                    },
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

    Ok(())
}

fn get_user_input(mouse_x: &mut i32, mouse_y: &mut i32) -> UserInput {
    // get mouse and keyboard input
    let ord = core::sync::atomic::Ordering::Relaxed;
    let up = UP.load(ord);
    let down = DOWN.load(ord);
    let left = LEFT.load(ord);
    let right = RIGHT.load(ord);
    let mut shooting = false;
    while let Some(event) = get_mouse_buffer().get_last_event() {
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
        shooting = shooting | event.is_left_click(); // shooting if left mouse button is pressed atleast once per tick
    }
    let map_mouse_x = *mouse_x as isize;
    let map_mouse_y = *mouse_y as isize;

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
        "{}Usage:{} game {}-s|--server -ip[xxx.xxx.xxx.xxx]{}",
        csi_title, csi_reset, csi_option, csi_reset
    );
    Ok(())
}

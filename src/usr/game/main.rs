use crate::api::console::Style;
use crate::api::process::ExitCode;
use crate::sys;
use crate::sys::console;
use crate::sys::keyboard::{DOWN, LEFT, RIGHT, UP};
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::game::UserInput;
use crate::usr::game::map::{Map};
use crate::usr::game::renderer;
use crate::usr::game::renderer::Color;
use crate::usr::game::state::{GUI_WIDTH, WALL_DENSITY};

//G640x480x16
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const SERVER_IP: &str = "192.168.0.1";
const SERVER_PORT: u16 = 1234;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    // map_demo();
    // return Ok(());
    if args.iter().any(|&arg| arg == "-h" || arg == "--help") {
        return help();
    }
    // let is_server = args.iter().any(|&arg| arg == "-s" || arg == "--server");
    // let ip = args.iter().find(|&&arg| arg.starts_with("-ip="));

    let mut game = crate::usr::game::game::Game::new(WIDTH, HEIGHT);
    game.init();
    kprintln!("Starting game...");

    let mut mouse_x: i32 = 0;
    let mut mouse_y: i32 = 0;
    let mut shooting = false;
    get_mouse_buffer().clear_events();

    loop {
        // parse input to game
        let user_input = get_user_input(&mut mouse_x, &mut mouse_y, &mut shooting);
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

    renderer.draw_map_buffer(map.clone());
    renderer.draw_map();
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

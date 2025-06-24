use crate::api::console::Style;
use crate::api::process::ExitCode;
use crate::sys;

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
    loop {
        // TODO get mouse and keyboard input
        // TODO parse input to game
        game.tick();
        game.draw();
        sys::clk::halt();
    }

    Ok(())
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
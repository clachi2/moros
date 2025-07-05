use alloc::vec::Vec;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::Player;

pub static PLAYER_SPEED: f64 = 40.0;
pub static PLAYER_SIZE: usize = 8;
pub static RESPAWN_TIME: f64 = 3.0; // in seconds

pub static BULLET_SPEED: f64 = 40.0;
pub static BULLET_TRAVEL_DIST: f64 = 200.0;
pub static SHOOTING_RATE_PER_SECOND: f64 = 5.0;
pub static MAX_AMMO: usize = 5;
pub static RELOAD_TIME: f64 = 1.0; // in seconds

pub static GUI_WIDTH: usize = 80;
pub static GUI_HEIGHT_PER_PLAYER: usize = 20;

pub static TICK_RATE: f64 = 64.0;
pub static WALL_DENSITY: f32 = 0.4;
pub static POINTS_PER_KILL: usize = 1;
pub static POINTS_PER_DEATH_MINUS: usize = 0;

pub(crate) trait Serializable {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Self;
} // TODO für alle structs implementieren

pub(crate) struct GameState {
    pub(crate) map: Map,
    pub(crate) players: Vec<Player>,
    pub(crate) bullets: Vec<Bullet>,
    pub(crate) current_player_index: usize,
    pub(crate) last_tick: f64, // Timestamp
}

impl GameState {
    pub fn new(map: Map) -> Self {
        GameState {
            map,
            players: Vec::new(),
            bullets: Vec::new(),
            current_player_index: 0,
            last_tick: 0.0, // Initialize with a default value
        }
    }
}


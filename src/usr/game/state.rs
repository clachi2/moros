use alloc::vec::Vec;
use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::Player;

pub static PLAYER_SPEED: f64 = 40.0; // Speed of player movement per tick
pub static PLAYER_SIZE: usize = 8; // Size of the player in pixels
pub static BULLET_SPEED: f64 = 40.0; // Speed of bullet movement per tick
pub static BULLET_TRAVEL_DIST: f64 = 200.0; // Maximum travel distance for bullets
pub static SHOOTING_RATE_PER_SECOND: f64 = 1000.0; // Maximum travel distance for bullets
pub static GUI_WIDTH: usize = 80;
pub static TICK_RATE: f64 = 64.0; // Number of ticks per second
pub static WALL_DENSITY: f32 = 0.5; // Density of walls in the map

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


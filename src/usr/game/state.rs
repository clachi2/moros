use crate::usr::game::bullet::Bullet;
use crate::usr::game::map::Map;
use crate::usr::game::player::Player;
use alloc::vec::Vec;

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

impl Serializable for GameState {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize map
        let map_data = self.map.serialize();
        result.extend_from_slice(&(map_data.len() as u32).to_le_bytes());
        result.extend(map_data);

        // Serialize players count and players
        result.extend_from_slice(&(self.players.len() as u32).to_le_bytes());
        for player in &self.players {
            let player_data = player.serialize();
            result.extend_from_slice(&(player_data.len() as u32).to_le_bytes());
            result.extend(player_data);
        }

        // Serialize bullets count and bullets
        result.extend_from_slice(&(self.bullets.len() as u32).to_le_bytes());
        for bullet in &self.bullets {
            let bullet_data = bullet.serialize();
            result.extend_from_slice(&(bullet_data.len() as u32).to_le_bytes());
            result.extend(bullet_data);
        }

        // Serialize current_player_index (4 bytes)
        result.extend_from_slice(&(self.current_player_index as u32).to_le_bytes());

        // Serialize last_tick (8 bytes)
        result.extend_from_slice(&self.last_tick.to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 20 {
            return GameState::new(Map::new(320, 200, 12, 10, 20));
        }

        let mut offset = 0;

        // Deserialize map
        let map_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        if data.len() < offset + map_len {
            return GameState::new(Map::new(320, 200, 12, 10, 20));
        }

        let map = Map::deserialize(&data[offset..offset + map_len]);
        offset += map_len;

        // Deserialize players
        if data.len() < offset + 4 {
            return GameState::new(map);
        }

        let players_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let mut players = Vec::new();
        for _ in 0..players_count {
            if data.len() < offset + 4 {
                break;
            }

            let player_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if data.len() < offset + player_len {
                break;
            }

            let player = Player::deserialize(&data[offset..offset + player_len]);
            players.push(player);
            offset += player_len;
        }

        // Deserialize bullets
        let mut bullets = Vec::new();
        if data.len() >= offset + 4 {
            let bullets_count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            for _ in 0..bullets_count {
                if data.len() < offset + 4 {
                    break;
                }

                let bullet_len = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]) as usize;
                offset += 4;

                if data.len() < offset + bullet_len {
                    break;
                }

                let bullet = Bullet::deserialize(&data[offset..offset + bullet_len]);
                bullets.push(bullet);
                offset += bullet_len;
            }
        }

        // Deserialize current_player_index
        let current_player_index = if data.len() >= offset + 4 {
            let index = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            index
        } else {
            0
        };

        // Deserialize last_tick
        let last_tick = if data.len() >= offset + 8 {
            f64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ])
        } else {
            0.0
        };

        GameState {
            map,
            players,
            bullets,
            current_player_index,
            last_tick,
        }
    }
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

use alloc::vec::Vec;
use crate::usr::game::state::{PLAYER_SPEED, Serializable};
use num_traits::Float;

pub(crate) struct UserInput {
    // keyboard input
    pub(crate) up: bool,
    pub(crate) right: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    // mouse input
    pub(crate) shooting: bool,
    pub(crate) map_mouse_x: isize,
    pub(crate) map_mouse_y: isize,
}

impl Clone for UserInput {
    fn clone(&self) -> Self {
        UserInput {
            up: self.up,
            right: self.right,
            down: self.down,
            left: self.left,
            shooting: self.shooting,
            map_mouse_x: self.map_mouse_x,
            map_mouse_y: self.map_mouse_y,
        }
    }
}

impl Serializable for UserInput {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Pack boolean inputs into a single byte
        let mut input_flags = 0u8;
        if self.up {
            input_flags |= 0b00001;
        }
        if self.right {
            input_flags |= 0b00010;
        }
        if self.down {
            input_flags |= 0b00100;
        }
        if self.left {
            input_flags |= 0b01000;
        }
        if self.shooting {
            input_flags |= 0b10000;
        }
        result.push(input_flags);

        // Serialize mouse coordinates (4 bytes each)
        result.extend_from_slice(&(self.map_mouse_x as i32).to_le_bytes());
        result.extend_from_slice(&(self.map_mouse_y as i32).to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 9 {
            return UserInput {
                up: false,
                right: false,
                down: false,
                left: false,
                shooting: false,
                map_mouse_x: 0,
                map_mouse_y: 0,
            };
        }

        let input_flags = data[0];
        let map_mouse_x = i32::from_le_bytes([data[1], data[2], data[3], data[4]]) as isize;
        let map_mouse_y = i32::from_le_bytes([data[5], data[6], data[7], data[8]]) as isize;

        UserInput {
            up: (input_flags & 0b00001) != 0,
            right: (input_flags & 0b00010) != 0,
            down: (input_flags & 0b00100) != 0,
            left: (input_flags & 0b01000) != 0,
            shooting: (input_flags & 0b10000) != 0,
            map_mouse_x,
            map_mouse_y,
        }
    }
}

pub(crate) struct Player {
    pub(crate) id: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) alive: bool,
    pub(crate) time_of_death: f64, // Timestamp of death (get using time::epoch_time())
    pub(crate) user_input: UserInput,
    pub(crate) pointing_to: (usize, usize),
    pub(crate) points: usize,
    pub(crate) ammo: usize,
    pub(crate) last_shot: f64, // Timestamp of the last shot (get using time::epoch_time())
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Player {
            id: self.id,
            x: self.x,
            y: self.y,
            alive: self.alive,
            time_of_death: self.time_of_death,
            user_input: self.user_input.clone(),
            pointing_to: self.pointing_to,
            points: self.points,
            ammo: self.ammo,
            last_shot: self.last_shot,
        }
    }
}

impl Player {
    pub fn next_wanted_position(&self, tick_delta: f64) -> (f64, f64) {
        let mut delta_x: f64 = 0.0;
        let mut delta_y: f64 = 0.0;
        if self.user_input.up {
            delta_y -= 1.0;
        }
        if self.user_input.down {
            delta_y += 1.0;
        }
        if self.user_input.left {
            delta_x -= 1.0;
        }
        if self.user_input.right {
            delta_x += 1.0;
        }
        // Normalize the direction vector to have length tick_delta * PLAYER_SPEED
        let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
        if length > 0.0 {
            let scale = (tick_delta * PLAYER_SPEED) / length;
            (self.x + delta_x * scale, self.y + delta_y * scale)
        } else {
            (self.x, self.y) // No movement
        }
    }
}

impl Serializable for Player {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Serialize player ID (4 bytes)
        result.extend_from_slice(&(self.id as u32).to_le_bytes());

        // Serialize position (8 bytes each for x and y as f64)
        result.extend_from_slice(&self.x.to_le_bytes());
        result.extend_from_slice(&self.y.to_le_bytes());

        // Serialize alive status (1 byte)
        result.push(if self.alive { 1 } else { 0 });

        // Serialize time_of_death (8 bytes)
        result.extend_from_slice(&self.time_of_death.to_le_bytes());

        // Serialize user input
        result.extend(self.user_input.serialize());

        // Serialize pointing_to (4 bytes each)
        result.extend_from_slice(&(self.pointing_to.0 as u32).to_le_bytes());
        result.extend_from_slice(&(self.pointing_to.1 as u32).to_le_bytes());


        // Serialize points (4 bytes)
        result.extend_from_slice(&(self.points as u32).to_le_bytes());

        // Serialize ammo (4 bytes)
        result.extend_from_slice(&(self.ammo as u32).to_le_bytes());

        // Serialize last_shot (8 bytes)
        result.extend_from_slice(&self.last_shot.to_le_bytes());

        result
    }

    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 58 {
            // Minimum size for all fields
            return Player {
                id: 0,
                x: 0.0,
                y: 0.0,
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
                points: 0,
                ammo: 0,
                last_shot: 0.0,
            };
        }

        let id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let x = f64::from_le_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);
        let y = f64::from_le_bytes([
            data[12], data[13], data[14], data[15], data[16], data[17], data[18], data[19],
        ]);
        let alive = data[20] != 0;
        let time_of_death = f64::from_le_bytes([
            data[21], data[22], data[23], data[24], data[25], data[26], data[27], data[28],
        ]);

        // Deserialize user input (9 bytes)
        let user_input = UserInput::deserialize(&data[29..38]);

        let pointing_to_x = u32::from_le_bytes([data[38], data[39], data[40], data[41]]) as usize;
        let pointing_to_y = u32::from_le_bytes([data[42], data[43], data[44], data[45]]) as usize;

        let points = u32::from_le_bytes([data[46], data[47], data[48], data[49]]) as usize;
        let ammo = u32::from_le_bytes([data[50], data[51], data[52], data[53]]) as usize;
        let last_shot = f64::from_le_bytes([
            data[54], data[55], data[56], data[57], data[58], data[59], data[60], data[61],
        ]);

        Player {
            id,
            x,
            y,
            alive,
            time_of_death,
            user_input,
            pointing_to: (pointing_to_x, pointing_to_y),
            points,
            ammo,
            last_shot,
        }
    }
}
use crate::usr::game::state::{PLAYER_SPEED};
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
    pub(crate) last_reload_ammo: f64, // Timestamp of the last ammo reload
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
            last_reload_ammo: 0.0, // Initialize with a default value
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
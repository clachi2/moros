use crate::sys::keyboard::{DOWN, LEFT, RIGHT, UP};
use crate::sys::mouse::get_mouse_buffer;
use crate::usr::game::tanks::state::Serializable;
use alloc::vec::Vec;

/// Stores user input state for keyboard (up, down, right, left) and mouse (x, y, clicking) actions.
pub struct UserInput {
    pub up: bool,
    pub right: bool,
    pub down: bool,
    pub left: bool,
    pub shooting: bool,
    pub map_mouse_x: isize,
    pub map_mouse_y: isize,
}

impl Clone for UserInput {
    /// Creates a new `UserInput` instance with the same state as `self`.
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
    /// Serializes the `UserInput` instance into a byte vector.
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

    /// Deserializes a byte slice into a `UserInput` instance.
    ///
    /// If the data is too short, it returns a default `UserInput` with all fields set to false and coordinates to 0.
    fn deserialize(data: &[u8]) -> Self {
        if data.len() < 9 {
            // 1 byte for input flags + 4 bytes for map_mouse_x + 4 bytes for map_mouse_y
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

impl UserInput {
    /// Updates the input states for an object based on user interactions from
    /// both the keyboard and the mouse. This function handles directional input,
    /// mouse movements, and shooting status based on mouse clicks.
    ///
    /// # Parameters
    ///
    /// * `width` - The width of the boundary or map, used to clamp mouse input to valid x-coordinates.
    /// * `height` - The height of the boundary or map, used to clamp mouse input to valid y-coordinates.
    pub fn update_from_io(&mut self, width: usize, height: usize) {
        // update user input from global keyboard state
        let ord = core::sync::atomic::Ordering::Relaxed;
        self.up = UP.load(ord);
        self.down = DOWN.load(ord);
        self.left = LEFT.load(ord);
        self.right = RIGHT.load(ord);

        // update mouse input from global mouse event queue
        let mut new_shooting = false;
        let mut atleast_once = false;
        while let Some(event) = get_mouse_buffer().get_last_event() {
            atleast_once = true;
            self.map_mouse_x += event.x_movement as isize;
            self.map_mouse_y += event.y_movement as isize;
            // Clamp mouse coordinates to the map boundaries
            if self.map_mouse_x < 0 {
                self.map_mouse_x = 0;
            } else if self.map_mouse_x >= width as isize {
                self.map_mouse_x = (width - 1) as isize;
            }
            if self.map_mouse_y < 0 {
                self.map_mouse_y = 0;
            } else if self.map_mouse_y >= height as isize {
                self.map_mouse_y = (height - 1) as isize;
            }
            new_shooting = new_shooting | event.is_left_click(); // shooting if left mouse button is pressed atleast once per tick
        }
        if atleast_once {
            self.shooting = new_shooting; // update shooting state only if there was a mouse event
        }
    }
}

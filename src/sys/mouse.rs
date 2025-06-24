use crate::sys;
use nolock::queues::mpmc;
use nolock::queues::mpmc::bounded::scq::{Receiver, Sender};
use spin::{Mutex, Once};
use x86_64::instructions::port::Port;

static MOUSE_BUFFER: Once<MouseQueue> = Once::new();
static MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());

pub fn get_mouse_buffer() -> &'static MouseQueue {
    MOUSE_BUFFER.call_once(|| MouseQueue::new())
}

pub fn init() {
    log!("Initializing Mouse");
    sys::idt::set_irq_handler(12, mouse_irq_handler);

    let mut command_port: Port<u8> = Port::new(0x64);
    let mut data_port: Port<u8> = Port::new(0x60);
    unsafe {
        command_port.write(0xA8); // Enable second PS/2 port (mouse)
        command_port.write(0x20); // Read configuration byte
        let config = data_port.read();
        command_port.write(0x60); // Write configuration byte
        data_port.write(config | 0x02); // Enable IRQ12 (mouse)
        command_port.write(0xD4); // Enable mouse
        data_port.write(0xF4); // Send enable command to mouse
    }
}

fn mouse_irq_handler() {
    let event    = MOUSE.lock().internal_irq_handler();
    if let Some(event) = event {
        get_mouse_buffer().push_event(event);
    }
}

pub struct MouseEvent {
    pub x_movement: i8,
    pub y_movement: i8,
    pub buttons: u8,
}

impl MouseEvent {
    pub fn is_left_click(&self) -> bool {
        self.buttons & 0x01 != 0
    }
    pub fn is_right_click(&self) -> bool {
        self.buttons & 0x02 != 0
    }
    pub fn is_middle_click(&self) -> bool {
        self.buttons & 0x04 != 0
    }
}

pub struct MouseQueue {
    receiver: Receiver<MouseEvent>,
    sender: Sender<MouseEvent>,
}

impl MouseQueue {
    fn new() -> MouseQueue {
        let (receiver, sender) = mpmc::bounded::scq::queue(128);
        MouseQueue { receiver, sender }
    }

    pub fn push_event(&self, key: MouseEvent) {
        if self.receiver.is_closed() {
            panic!("MouseQueue is closed!");
        }
        self.sender.try_enqueue(key).ok();
    }

    pub fn get_last_event(&self) -> Option<MouseEvent> {
        if self.receiver.is_closed() {
            panic!("MouseQueue is closed!");
        }
        match self.receiver.try_dequeue() {
            Ok(key) => Some(key),
            Err(_) => None,
        }
    }

    pub fn wait_for_event(&self) -> MouseEvent {
        if self.receiver.is_closed() {
            panic!("MouseQueue is closed!");
        }
        loop {
            match self.receiver.try_dequeue() {
                Ok(key) => return key,
                Err(_) => {}
            }
        }
    }

    pub fn clear_events(&self) {
        if self.receiver.is_closed() {
            panic!("MouseQueue is closed!");
        }
        while let Ok(_) = self.receiver.try_dequeue() {}
    }
}

struct Mouse {}

impl Mouse {
    pub const fn new() -> Mouse {
        Mouse {}
    }

    fn read_mouse_data(&self) -> Option<[u8; 3]> {
        let mut command_port: Port<u8> = Port::new(0x64);
        let mut data_port: Port<u8> = Port::new(0x60);

        unsafe {
            if (command_port.read() & 0x20) == 0 {
                return None;
            }
            let byte0 = data_port.read();
            if (byte0 & 0x08) == 0 {
                return None;
            }

            if (command_port.read() & 0x20) == 0 {
                return None;
            }
            let byte1 = data_port.read();

            if (command_port.read() & 0x20) == 0 {
                return None;
            }
            let byte2 = data_port.read();

            Some([byte0, byte1, byte2])
        }
    }

    fn internal_irq_handler(&self) -> Option<MouseEvent> {
        // println!("Mouse IRQ handler called");
        if let Some(mouse_data) = self.read_mouse_data() {
            Some(MouseEvent {
                x_movement: mouse_data[1] as i8,
                y_movement: -(mouse_data[2] as i8),
                buttons: mouse_data[0] & 0x07,
            })
        } else {
            None
        }
    }
}

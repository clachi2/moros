use alloc::format;
use crate::api::fs;
use crate::sys::net::socket::udp::UdpSocket;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use core::str::FromStr;
use nolock::queues::mpmc;
use nolock::queues::mpmc::bounded::scq::{Receiver, Sender};
use smoltcp::socket::udp;
use smoltcp::socket::udp::UdpMetadata;
use smoltcp::wire::{IpAddress, IpCidr};
use spin::{Mutex, Once};
use crate::api::fs::{FileIO, IO};
use crate::sys::net::socket::SOCKETS;

const BUFFER_SIZE: usize = 1024;

static SENDING_BUFFER: Once<MessageQueue> = Once::new();
static RECEIVING_BUFFER: Once<MessageQueue> = Once::new();

fn get_sending_buffer() -> &'static MessageQueue {
    SENDING_BUFFER.call_once(|| MessageQueue::new())
}

fn get_receiving_buffer() -> &'static MessageQueue {
    RECEIVING_BUFFER.call_once(|| MessageQueue::new())
}

pub struct NetworkHandler {
    socket: UdpSocket,
    buffer: [u8; 1024],
    is_server: bool,
    remotes : Vec<UdpMetadata>,
}

impl NetworkHandler {

    pub fn new() -> Self {
        NetworkHandler {
            socket: UdpSocket::new(),
            buffer: [0; 1024],
            is_server: false,
            remotes: Vec::new(),
        }
    }

    pub fn init(&mut self, is_server: bool, ip: Option<&str>) -> Result<(), String> {
        self.is_server = is_server;
        if self.is_server {
            self.set_ip("192.168.0.1").expect("TODO: panic message");
            if self.socket.listen(1234).is_err() {
                panic!("Failed to set up server socket");
            }

        }
        else {
            if self.socket.connect(IpAddress::from(Ipv4Addr::new(192, 168, 0, 1)), 1234).is_err() {
                return Err("Failed to connect to server".to_string());
            }
        }

        Ok(())

    }

    pub fn set_server(&mut self) {
        self.is_server = true;
    }

    pub fn set_ip(&mut self, ip: &str) -> Result<(), String> {
        if let ipv4 = Ipv4Addr::from_str(ip) {
            let addr = IpAddress::from(ipv4.unwrap());
            return if fs::write("/dev/net/ip", IpCidr::new(addr, 24).to_string().as_bytes())
                .is_err()
            {
                Err("Failed to set IP address".to_string())
            } else {
                Ok(())
            };
        }
        if fs::write("/dev/net/gw", b"192.168.0.1").is_err() {
            return Err("Failed to set gateway".to_string());
        }
        if fs::write("/ini/dns", b"192.168.0.3").is_err() {
            return Err("Failed to set DNS".to_string());
        }
        Ok(())
    }

    // pub fn poll(&mut self) -> Result<(), String> {
    //     while self.socket.poll(IO::Read){
    //         if let Ok((size, remote_endpoint)) = {
    //             let mut sockets = SOCKETS.lock();
    //             let socket = sockets.get_mut::<udp::Socket>(self.socket.handle);
    //             socket.recv_slice(&mut *self.buffer).map_err(|_| ())
    //         } {
    //             let msg = &self.buffer[..size];
    //             println!("Received from {:?}: {:?}", remote_endpoint,
    //                 core::str::from_utf8(msg).unwrap_or("???"));
    //
    //             // Prepare response
    //             let response = format!("Server response to {:?}", remote_endpoint);
    //
    //             // Send response back to same endpoint
    //             if self.socket.poll(IO::Write) {
    //                 let mut sockets = SOCKETS.lock();
    //                 let socket = sockets.get_mut::<udp::Socket>(self.socket.handle);
    //                 socket.send_slice(response.as_bytes(), remote_endpoint).ok();
    //             }
    //         }
    //     }
    //
    //     Ok(())
    // }

    pub fn connect(&mut self, ip: &str, port: u16) -> Result<(), String> {
        let addr = IpAddress::from_str(ip).map_err(|_| "Invalid IP address".to_string())?;
        if self.socket.connect(addr, port).is_err() {
            return Err("Failed to connect to server".to_string());
        }
        Ok(())
    }

    pub fn send_message(&mut self, message: &str) -> Result<(), String> {

        Ok(())
    }



}

pub struct Message {
    data: [u8; BUFFER_SIZE],
    size: usize,
    metadata: UdpMetadata,
}

pub struct MessageQueue {
    receiver: Receiver<Message>,
    sender: Sender<Message>,
}

impl MessageQueue {
    pub fn new() -> Self {
        let (receiver, sender) = mpmc::bounded::scq::queue(BUFFER_SIZE);
        MessageQueue { receiver, sender }
    }
    pub fn push_message(&self, message: Message) {
        if self.receiver.is_closed() {
            panic!("MessageQueue is closed!");
        }
        if let Err(_) = self.sender.try_enqueue(message) {
            panic!("MessageQueue is full!");

        }
    }

    pub fn get_last_message(&self) -> Option<Message> {
        if self.receiver.is_closed() {
            panic!("MessageQueue is closed!");
        }
        match self.receiver.try_dequeue() {
            Ok(message) => Some(message),
            Err(_) => None,
        }
    }

    pub fn wait_for_message(&self) -> Message {
        if self.receiver.is_closed() {
            panic!("MessageQueue is closed!");
        }
        loop {
            match self.receiver.try_dequeue() {
                Ok(message) => return message,
                Err(_) => {}
            }
        }
    }

    pub fn clear_messages(&self) {
        if self.receiver.is_closed() {
            panic!("MessageQueue is closed!");
        }
        while let Ok(_) = self.receiver.try_dequeue() {}
    }
}

use crate::api::fs;
use crate::api::fs::{FileIO, IO};
use crate::kprintln;
use crate::sys::net::socket::SOCKETS;
use crate::sys::net::socket::udp::UdpSocket;
use crate::usr::game::map::Map;
use crate::usr::game::state::Serializable;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use core::str::FromStr;
use nolock::queues::mpmc;
use nolock::queues::mpmc::bounded::scq::{Receiver, Sender};
use smoltcp::socket::udp;
use smoltcp::socket::udp::UdpMetadata;
use smoltcp::wire::{IpAddress, IpCidr};
use spin::Once;

const BUFFER_SIZE: usize = 8192;

static SENDING_BUFFER: Once<MessageQueue> = Once::new();
static RECEIVING_BUFFER: Once<MessageQueue> = Once::new();

fn get_sending_buffer() -> &'static MessageQueue {
    SENDING_BUFFER.call_once(|| MessageQueue::new())
}

fn get_receiving_buffer() -> &'static MessageQueue {
    RECEIVING_BUFFER.call_once(|| MessageQueue::new())
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Connect,
    MapRequest,
    MapData,
    PlayerInput,
    GameState,
    GameStateUpdate,
}

impl MessageType {
    pub fn to_u8(&self) -> u8 {
        match self {
            MessageType::Connect => 0,
            MessageType::MapRequest => 1,
            MessageType::MapData => 2,
            MessageType::PlayerInput => 3,
            MessageType::GameState => 4,
            MessageType::GameStateUpdate => 5,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MessageType::Connect),
            1 => Some(MessageType::MapRequest),
            2 => Some(MessageType::MapData),
            3 => Some(MessageType::PlayerInput),
            4 => Some(MessageType::GameState),
            5 => Some(MessageType::GameStateUpdate),
            _ => None,
        }
    }
}

pub struct NetworkHandler {
    socket: UdpSocket,
    buffer: [u8; 8192],
    is_server: bool,
    connected_clients: Vec<UdpMetadata>,
    current_map: Option<Map>,
}

impl NetworkHandler {
    pub fn new() -> Self {
        NetworkHandler {
            socket: UdpSocket::new(),
            buffer: [0; 8192],
            is_server: false,
            connected_clients: Vec::new(),
            current_map: None,
        }
    }

    pub fn init(&mut self, is_server: bool, ip: Option<&str>) -> Result<(), String> {
        self.is_server = is_server;
        if is_server {
            self.set_ip("192.168.0.1").expect("TODO: panic message");
            if self.socket.listen(1234).is_err() {
                panic!("Failed to set up server socket");
            }
        } else {
            // Set client IP if provided
            if let Some(client_ip) = ip {
                self.set_ip(client_ip)?;
                kprintln!("Client IP set to: {}", client_ip);
            }

            if self
                .socket
                .connect(IpAddress::from(Ipv4Addr::new(192, 168, 0, 1)), 1234)
                .is_err()
            {
                return Err("Failed to connect to server".to_string());
            } else {
                kprintln!("Connected to server");
                // Send connection request
                // TODO einfach 100 mal schicken gerade .. brauche noch ack
                for _ in 0..100 {
                    self.send_message_type(MessageType::Connect, &[])?;
                }
            }
        }

        Ok(())
    }

    pub fn set_server(&mut self) {
        self.is_server = true;
    }

    pub fn set_ip(&mut self, ip: &str) -> Result<(), String> {
        let ipv4 = Ipv4Addr::from_str(ip).map_err(|_| "Invalid IP address format".to_string())?;
        let addr = IpAddress::from(ipv4);

        if fs::write("/dev/net/ip", IpCidr::new(addr, 24).to_string().as_bytes()).is_err() {
            return Err("Failed to set IP address".to_string());
        }
        if fs::write("/dev/net/gw", b"192.168.0.1").is_err() {
            return Err("Failed to set gateway".to_string());
        }
        if fs::write("/ini/dns", b"192.168.0.3").is_err() {
            return Err("Failed to set DNS".to_string());
        }
        Ok(())
    }

    pub fn connect(&mut self, ip: &str, port: u16) -> Result<(), String> {
        let addr = IpAddress::from_str(ip).map_err(|_| "Invalid IP address".to_string())?;
        if self.socket.connect(addr, port).is_err() {
            return Err("Failed to connect to server".to_string());
        }
        Ok(())
    }

    pub fn send_message(&mut self, message: &str) -> Result<(), String> {
        if self.socket.poll(IO::Write) {
            if self.socket.write(message.as_bytes()).is_err() {
                return Err("Failed to send message".to_string());
            }
        }
        Ok(())
    }

    pub fn send_message_type(&mut self, msg_type: MessageType, data: &[u8]) -> Result<(), String> {
        let mut message = Vec::new();
        message.push(msg_type.to_u8());
        message.extend_from_slice(data);

        if self.socket.poll(IO::Write) {
            if self.socket.write(&message).is_err() {
                return Err("Failed to send typed message".to_string());
            }
        }
        Ok(())
    }

    pub fn send_map(&mut self, map: &Map) -> Result<(), String> {
        let map_data = map.serialize();
        self.send_message_type(MessageType::MapData, &map_data)
    }

    pub fn poll_messages(&mut self) -> Result<Vec<(MessageType, Vec<u8>, UdpMetadata)>, String> {
        let mut messages = Vec::new();

        while self.socket.poll(IO::Read) {
            if let Ok((size, remote_endpoint)) = {
                let mut sockets = SOCKETS.lock();
                let socket = sockets.get_mut::<udp::Socket>(self.socket.handle);
                socket.recv_slice(&mut self.buffer).map_err(|_| ())
            } {
                if size > 0 {
                    if let Some(msg_type) = MessageType::from_u8(self.buffer[0]) {
                        let data = self.buffer[1..size].to_vec();

                        // DEBUG
                        // kprintln!(
                        //     "Received message of type {:?} from {:?}: {:?}",
                        //     msg_type,
                        //     remote_endpoint,
                        //     core::str::from_utf8(&data).unwrap_or("Invalid UTF-8")
                        // );

                        if self.is_server {
                            self.handle_server_message(msg_type.clone(), &data, remote_endpoint)?;
                        }

                        messages.push((msg_type, data, remote_endpoint));
                    }
                }
            }
        }

        Ok(messages)
    }

    //TODO : ist gerade bisschen doppelt aber networkhandler muss wissen an wen er was schickt wenn server
    fn handle_server_message(
        &mut self,
        msg_type: MessageType,
        data: &[u8],
        client: UdpMetadata,
    ) -> Result<(), String> {
        match msg_type {
            MessageType::Connect => {
                // Add client if not already connected
                if !self
                    .connected_clients
                    .iter()
                    .any(|c| c.endpoint == client.endpoint)
                {
                    self.connected_clients.push(client);
                    kprintln!("New client connected: {:?}", client.endpoint);
                }
            }
            MessageType::MapRequest => {
                // Send map to requesting client
                if let Some(ref map) = self.current_map {
                    let map_data = map.serialize();
                    self.send_map_to_client(&map_data, client)?;
                }
            }
            MessageType::PlayerInput => {
                // Extract player ID from sender's IP
                let player_id = match client.endpoint.addr {
                    smoltcp::wire::IpAddress::Ipv4(ipv4) => ipv4.octets()[3] as usize,
                    _ => 0,
                };
                kprintln!("Received input from player {}", player_id);
                if !data.is_empty() {
                    //TODO PROBLEM MIT ID REF
                    self.deserialize_user_input(player_id, &data);
                    //kprintln!("Received input from player {}", player_id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn send_map_to_client(&mut self, map_data: &[u8], client: UdpMetadata) -> Result<(), String> {
        let mut message = Vec::new();
        message.push(MessageType::MapData.to_u8());
        message.extend_from_slice(map_data);

        if self.socket.poll(IO::Write) {
            let mut sockets = SOCKETS.lock();
            let socket = sockets.get_mut::<udp::Socket>(self.socket.handle);
            if socket.send_slice(&message, client).is_err() {
                return Err("Failed to send map to client".to_string());
            }
        }
        Ok(())
    }

    pub fn set_map(&mut self, map: Map) {
        self.current_map = Some(map);
    }

    pub fn get_received_map(&mut self) -> Option<Map> {
        // Poll for messages and check for map data
        if let Ok(messages) = self.poll_messages() {
            for (msg_type, data, _) in messages {
                if let MessageType::MapData = msg_type {
                    if !data.is_empty() {
                        return Some(Map::deserialize(&data));
                    }
                }
            }
        }
        None
    }

    pub fn send_player_input(&mut self, input_data: &[u8]) -> Result<(), String> {
        self.send_message_type(MessageType::PlayerInput, input_data)
    }

    pub fn broadcast_game_state(&mut self, state_data: &[u8]) -> Result<(), String> {
        // kprintln!(
        //     "Broadcasting game state of size: {} bytes",
        //     state_data.len()
        // );

        // Split into chunks if too large
        const MAX_CHUNK_SIZE: usize = 1400; // Safe UDP packet size

        if state_data.len() > MAX_CHUNK_SIZE {
            kprintln!(
                "Game state too large ({}), splitting into chunks",
                state_data.len()
            );

            let chunks: Vec<&[u8]> = state_data.chunks(MAX_CHUNK_SIZE).collect();
            for client in self.connected_clients.clone() {
                if let Err(e) = self.send_chunked_game_state_to_client(&chunks, client) {
                    kprintln!(
                        "Failed to send chunked game state to client {:?}: {}",
                        client.endpoint,
                        e
                    );
                }
            }
        } else {
            for client in self.connected_clients.clone() {
                if let Err(e) = self.send_game_state_to_client(state_data, client) {
                    kprintln!(
                        "Failed to send game state to client {:?}: {}",
                        client.endpoint,
                        e
                    );
                }
            }
        }
        Ok(())
    }

    fn send_game_state_to_client(
        &mut self,
        state_data: &[u8],
        client: UdpMetadata,
    ) -> Result<(), String> {
        let mut message = Vec::new();
        message.push(MessageType::GameStateUpdate.to_u8());
        message.extend_from_slice(state_data);

        if self.socket.poll(IO::Write) {
            let mut sockets = SOCKETS.lock();
            let socket = sockets.get_mut::<udp::Socket>(self.socket.handle);
            if socket.send_slice(&message, client).is_err() {
                return Err("Failed to send game state to client".to_string());
            }
        }
        Ok(())
    }

    fn send_chunked_game_state_to_client(
        &mut self,
        chunks: &[&[u8]],
        client: UdpMetadata,
    ) -> Result<(), String> {
        // For now, just send the first chunk to avoid complexity
        // In a full implementation, you'd need chunk reassembly
        kprintln!("Sending only first chunk to avoid complexity");
        if let Some(first_chunk) = chunks.first() {
            self.send_game_state_to_client(first_chunk, client)?;
        }
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

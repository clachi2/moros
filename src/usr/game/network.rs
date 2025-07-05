use crate::api::fs;
use crate::api::fs::{FileIO, IO};
use crate::kprintln;
use crate::sys::net::socket::SOCKETS;
use crate::sys::net::socket::udp::UdpSocket;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use core::str::FromStr;
use smoltcp::socket::udp;
use smoltcp::socket::udp::UdpMetadata;
use smoltcp::wire::{IpAddress, IpCidr};

const BUFFER_SIZE: usize = 8192;

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
    current_map: Vec<u8>,
}

impl NetworkHandler {
    pub fn new() -> Self {
        NetworkHandler {
            socket: UdpSocket::new(),
            buffer: [0; 8192],
            is_server: false,
            connected_clients: Vec::new(),
            current_map: Vec::new(),
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
                // TODO einfach 1000 mal schicken gerade .. brauche noch ack
                for _ in 0..1000 {
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

    pub fn send_map(&mut self) -> Result<(), String> {
        let map_data = if self.current_map.is_empty() {
            kprintln!("No map set, sending empty map");
            Vec::new()
        } else {
            self.current_map.clone()
        };
        self.send_message_type(MessageType::MapData, &map_data)
            .map_err(|e| format!("Failed to send map: {}", e))
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
                if !self.current_map.is_empty() {
                    let map_data = self.current_map.clone();
                    self.send_map_to_client(&map_data, client)
                        .map_err(|e| format!("Failed to send map to client: {}", e))?;
                }
            }
            MessageType::PlayerInput => {
                // Player input handling is now done in handle_network_messages
                let _ = data; // Silence unused variable warning
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

    pub fn set_map(&mut self, map: Vec<u8>) {
        self.current_map = map;
    }

    pub fn get_received_map(&mut self) -> Option<Vec<u8>> {
        // Poll for messages and check for map data
        if let Ok(messages) = self.poll_messages() {
            for (msg_type, data, _) in messages {
                if let MessageType::MapData = msg_type {
                    if !data.is_empty() {
                        kprintln!("Received map data of size: {} bytes", data.len());
                        self.set_map(data.clone());
                        return Some(data);
                    } else {
                        //todo nervt wenn local spielt
                        //kprintln!("Received empty map data");
                    }
                }
            }
        }
        //todo nervt wenn local spielt
        //kprintln!("Didnt receive map data, returning current map");
        // If no map data received, return current map
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

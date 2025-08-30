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
    Disconnect,
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
            MessageType::Disconnect => 6,
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
            6 => Some(MessageType::Disconnect),
            _ => None,
        }
    }
}

pub struct NetworkHandler {
    socket: UdpSocket,
    buffer: [u8; BUFFER_SIZE],
    is_server: bool,
    connected_clients: Vec<UdpMetadata>,
    current_map: Vec<u8>,
}

impl NetworkHandler {
    /// Creates a new instance of `NetworkHandler` with default values.
    ///
    /// This constructor initializes a new network handler with an empty UDP socket
    ///
    /// # Returns
    /// A new `NetworkHandler` instance with all fields initialized to their default values.
    pub fn new() -> Self {
        NetworkHandler {
            socket: UdpSocket::new(),
            buffer: [0; 8192],
            is_server: false,
            connected_clients: Vec::new(),
            current_map: Vec::new(),
        }
    }

    /// Initializes the network handler for either server or client mode.
    ///
    /// This method sets up the network handler based on the specified role. For server mode,
    /// it configures a fixed IP address (192.168.0.1) and starts listening on port 1234.
    /// For client mode, it optionally sets a custom IP address and attempts to connect to
    /// the server, sending multiple connection requests to ensure reliable connection establishment.
    ///
    /// # Parameters
    /// - `is_server`: A boolean indicating whether this instance should operate as a server (`true`) or client (`false`)
    /// - `ip`: An optional IP address string for client mode. If `None`, the default client IP is used
    ///
    /// # Returns
    /// - `Ok(())` if initialization was successful
    /// - `Err(String)` if initialization failed, with an error description
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
                // 1000 is enough with up to 9 players as presented
                for _ in 0..1000 {
                    self.send_message_type(MessageType::Connect, &[])?;
                }
            }
        }

        Ok(())
    }

    /// Sets the network handler to server mode.
    pub fn set_server(&mut self) {
        self.is_server = true;
    }

    /// Configures the network interface with the specified IP address and related settings.
    ///
    /// This method sets up the network configuration by writing the IP address, gateway,
    /// and DNS settings to the appropriate system files. The IP is configured with a /24
    /// subnet mask, and default gateway and DNS server addresses are set.
    ///
    /// # Parameters
    /// - `ip`: A string slice containing the IP address to configure
    ///
    /// # Returns
    /// - `Ok(())` if the network configuration was set successfully
    /// - `Err(String)` if any of the network configuration steps failed
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

    /// Sends a typed message with a specific message type and payload data.
    ///
    /// This method constructs a message by prepending the message type identifier
    /// to the provided data payload and sends it over the network. The message format
    /// consists of a single byte for the message type followed by the data bytes.
    ///
    /// # Parameters
    /// - `msg_type`: The type of message to send, which will be encoded as the first byte
    /// - `data`: A byte slice containing the payload data to send with the message
    ///
    /// # Returns
    /// - `Ok(())` if the message was sent successfully
    /// - `Err(String)` if sending failed, with an error description
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

    /// Polls for incoming messages and processes them based on the handler's role.
    ///
    /// This method continuously reads available messages from the UDP socket, parsing
    /// each message to extract the message type and payload data. If this handler is
    /// operating in server mode, it also processes certain message types automatically
    /// (such as connection requests and map requests). All received messages are returned
    /// for further processing by the caller.
    ///
    /// # Returns
    /// - `Ok(Vec<(MessageType, Vec<u8>, UdpMetadata)>)` containing a vector of tuples with:
    ///   - `MessageType`: The type of the received message
    ///   - `Vec<u8>`: The payload data of the message
    ///   - `UdpMetadata`: Network metadata about the sender
    /// - `Err(String)` if message polling failed, with an error description
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

    /// Handles incoming messages when operating in server mode.
    ///
    /// This method processes specific message types that require server-side handling,
    /// such as client connection requests and map data requests. It maintains the list
    /// of connected clients and responds to client requests appropriately.
    ///
    /// # Parameters
    /// - `msg_type`: The type of message received from the client
    /// - `data`: The payload data of the received message
    /// - `client`: Network metadata identifying the client that sent the message
    ///
    /// # Returns
    /// - `Ok(())` if the message was handled successfully
    /// - `Err(String)` if message handling failed, with an error description
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
            _ => {}
        }
        Ok(())
    }

    /// Sends map data to a specific client.
    ///
    /// This method constructs a map data message and sends it directly to the specified
    /// client. The message format includes the MapData message type identifier followed
    /// by the serialized map data.
    ///
    /// # Parameters
    /// - `map_data`: A byte slice containing the serialized map data to send
    /// - `client`: Network metadata identifying the target client
    ///
    /// # Returns
    /// - `Ok(())` if the map data was sent successfully
    /// - `Err(String)` if sending failed, with an error description
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

    /// Sets the current map data for this network handler.
    pub fn set_map(&mut self, map: Vec<u8>) {
        self.current_map = map;
    }

    /// Attempts to retrieve map data from incoming messages.
    ///
    /// This method polls for incoming messages and specifically looks for MapData
    /// message types. If map data is received, it updates the internal map storage
    /// and returns the map data. This method is typically used by clients to receive
    /// map data from the server.
    ///
    /// # Returns
    /// - `Some(Vec<u8>)` containing the received map data if a MapData message was found
    /// - `None` if no map data was received in the current polling cycle
    pub fn get_received_map(&mut self) -> Option<Vec<u8>> {
        // Poll for messages and check for map data
        if let Ok(messages) = self.poll_messages() {
            for (msg_type, data, _) in messages {
                if let MessageType::MapData = msg_type {
                    if !data.is_empty() {
                        kprintln!("Received map data of size: {} bytes", data.len());
                        self.set_map(data.clone());
                        return Some(data);
                    }
                }
            }
        }
        //Only for debugging purposes
        //kprintln!("Didnt receive map data, returning current map");
        None
    }

    /// Sends player input data to the connected server or clients.
    pub fn send_player_input(&mut self, input_data: &[u8]) -> Result<(), String> {
        self.send_message_type(MessageType::PlayerInput, input_data)
    }

    /// Broadcasts the current game state to all connected clients.
    ///
    /// This method sends game state data to all clients currently connected to the server.
    /// If the state data exceeds the safe UDP packet size (1400 bytes), it splits the data
    /// into chunks and sends only the first chunk to avoid network fragmentation issues.
    /// This method is typically called by the server to synchronize game state across all clients.
    ///
    /// # Parameters
    /// - `state_data`: A byte slice containing the serialized game state data to broadcast
    ///
    /// # Returns
    /// - `Ok(())` if the game state was broadcast successfully to all clients
    /// - `Err(String)` if broadcasting failed, with an error description
    pub fn broadcast_game_state(&mut self, state_data: &[u8]) -> Result<(), String> {
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

    /// Sends game state data to a specific client.
    ///
    /// This method constructs a GameStateUpdate message containing the provided state data
    /// and sends it directly to the specified client. The message format includes the
    /// GameStateUpdate message type identifier followed by the serialized game state data.
    ///
    /// # Parameters
    /// - `state_data`: A byte slice containing the serialized game state data to send
    /// - `client`: Network metadata identifying the target client
    ///
    /// # Returns
    /// - `Ok(())` if the game state was sent successfully
    /// - `Err(String)` if sending failed, with an error description
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

    /// Sends chunked game state data to a specific client.
    ///
    /// This method handles large game state data by sending it in chunks to avoid
    /// UDP packet size limitations. Currently, it implements a simplified approach
    /// by sending only the first chunk to avoid complexity in chunk reassembly.
    /// This is a fallback mechanism when game state data exceeds safe UDP packet sizes.
    ///
    /// # Parameters
    /// - `chunks`: A slice of byte slices, each representing a chunk of the game state data
    /// - `client`: Network metadata identifying the target client
    ///
    /// # Returns
    /// - `Ok(())` if the chunked data was sent successfully
    /// - `Err(String)` if sending failed, with an error description
    fn send_chunked_game_state_to_client(
        &mut self,
        chunks: &[&[u8]],
        client: UdpMetadata,
    ) -> Result<(), String> {
        kprintln!("Sending only first chunk to avoid complexity");
        if let Some(first_chunk) = chunks.first() {
            self.send_game_state_to_client(first_chunk, client)?;
        }
        Ok(())
    }
}

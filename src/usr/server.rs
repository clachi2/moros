use crate::api::fs;
use crate::api::fs::{FileIO, IO};
use crate::api::process::ExitCode;
use crate::sys;
use crate::sys::net::socket::tcp::TcpSocket;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::net::Ipv4Addr;
use smoltcp::socket::udp;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::sys::net::socket::SOCKETS;
use crate::sys::net::socket::udp::UdpSocket;

extern crate alloc;

fn run_server2(port: u16) {
    let mut socket = UdpSocket::new();
    socket.listen(port).unwrap();

    let mut buf = [0u8; 1024];
    let mut count = 0;

    loop {
        if socket.poll(IO::Read) {
            // Receive message and get sender's endpoint
            if let Ok((size, remote_endpoint)) = {
                let mut sockets = SOCKETS.lock();
                let socket = sockets.get_mut::<udp::Socket>(socket.handle);
                socket.recv_slice(&mut buf).map_err(|_| ())
            } {
                let msg = &buf[..size];
                println!("Received from {:?}: {:?}", remote_endpoint,
                    core::str::from_utf8(msg).unwrap_or("???"));

                // Prepare response
                let response = format!("Server response {} to {:?}", count, remote_endpoint);
                count += 1;

                // Send response back to same endpoint
                if socket.poll(IO::Write) {
                    let mut sockets = SOCKETS.lock();
                    let socket = sockets.get_mut::<udp::Socket>(socket.handle);
                    socket.send_slice(response.as_bytes(), remote_endpoint).ok();
                }
            }
        }
        sys::clk::halt();
    }
}

fn run_server(port: u16) {
    let mut listener = UdpSocket::new();
    listener.listen(port).unwrap();

    let mut clients: Vec<UdpSocket> = Vec::new();

    let mut count = 0;

    loop {
        // Check for new connection
        if listener.poll(IO::Read) {
            if let Ok(remote_ip) = listener.accept() {
                println!("New connection from {:?}", remote_ip);

                let connected_socket = core::mem::replace(&mut listener, UdpSocket::new());
                clients.push(connected_socket);

                // Replace listener so it can accept again
                listener.listen(port).unwrap();
            }
        }

        // Handle connected clients
        for client in clients.iter_mut() {
            while client.poll(IO::Read) {
                let mut buf = [0u8; 1024];
                if let Ok(size) = client.read(&mut buf) {
                    if size > 0 {
                        let msg = &buf[..size];
                        println!(
                            "Client sent: {:?}",
                            core::str::from_utf8(msg).unwrap_or("???")
                        );

                        if client.poll(IO::Write) {
                            // Here we can send a response back to the client
                            println!("Sending response to client...");
                            // For example, we can send a simple "pong" message
                            let reply = format!("Answer from Server with count: {}" , count);
                            client.write(reply.as_bytes()).ok();
                            count += 1;
                        }
                    }
                }
            }
        }

        sys::clk::halt();
    }
}

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    let addr = IpAddress::from(Ipv4Addr::new(192, 168, 0, 1));

    if fs::write("/dev/net/ip", IpCidr::new(addr, 24).to_string().as_bytes()).is_err() {
        println!("Fehler: IP-Adresse konnte nicht gesetzt werden.");
        return Err(ExitCode::Failure);
    }
    fs::write("/dev/net/gw", b"192.168.0.1").ok(); // Gateway auf Server-IP ist eine gängige Konfiguration.
    fs::write("/ini/dns", b"192.168.0.3").is_ok(); // DNS wird nicht benötigt.

    println!("TCP Server startet und behandelt Clients sequenziell.");
    println!("IP konfiguriert auf {}", addr);

    // run_server(1234);
    run_server2(1234);

    Ok(())
}

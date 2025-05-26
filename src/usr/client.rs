use alloc::string::ToString;
use core::net::Ipv4Addr;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::api::fs;
use crate::api::fs::FileIO;
use crate::api::process::ExitCode;
use crate::sys::net::socket::tcp::TcpSocket;

extern crate alloc;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {

    let addr = IpAddress::from(Ipv4Addr::new(192, 168, 0, 2));
    let server_ip = IpAddress::from(Ipv4Addr::new(192, 168, 0, 1));

    if fs::write("/dev/net/ip", IpCidr::new(addr, 24).to_string().as_bytes()).is_err() {
        println!("Fehler: IP-Adresse konnte nicht gesetzt werden.");
        return Err(ExitCode::Failure);
    }
    fs::write("/dev/net/gw", b"192.168.0.1").ok(); // Gateway auf Server-IP ist eine gängige Konfiguration.
    fs::write("/ini/dns", b"192.168.0.3").is_ok(); // DNS wird nicht benötigt.

    let server_port = 1234;

    println!("Connecting to TCP server on {}:{}", server_ip, server_port);

    let mut client = TcpSocket::new();


    if let Err(e) = client.connect(server_ip, server_port) {
        println!("Failed to connect to server: {:?}", e);
        return Err(ExitCode::Failure);
    }

    println!("Connected to server.");

    // Nachricht empfangen
    let mut buf = [0u8; 128]; // Buffer eventuell etwas größer machen, falls die Server-Nachricht länger ist
    match client.read(&mut buf) {
        Ok(n) => {
            if n > 0 {
                println!("Read {} bytes from server.", n);
                // Gib die empfangene Nachricht aus:
                match core::str::from_utf8(&buf[..n]) {
                    Ok(message) => println!("Received: {}", message.trim_end()), // trim_end() um eventuelle \n zu entfernen
                    Err(_) => println!("Received (raw bytes): {:?}", &buf[..n]),
                }
            } else {
                println!("Server closed the connection without sending data.");
            }
        }
        Err(e) => {
            println!("Failed to read from server: {:?}", e);

        }
    }

    client.close();
    println!("Connection closed.");

    Ok(())
}
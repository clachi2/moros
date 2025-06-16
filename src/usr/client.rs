use crate::api::fs;
use crate::api::fs::{FileIO, IO};
use crate::api::process::ExitCode;
use crate::sys::net::SocketStatus;
use crate::sys::net::socket::tcp::TcpSocket;
use crate::sys::syscall::service::exit;
use alloc::format;
use alloc::string::ToString;
use bit_field::BitField;
use core::net::Ipv4Addr;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::sys::net::socket::udp::UdpSocket;

extern crate alloc;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    let mut last_digit = 2;
    if args.len() > 1 {
        last_digit = match args[1].parse::<u8>() {
            Ok(digit) if digit >= 2 && digit <= 255 => digit,
            _ => {
                println!("Ungültige Eingabe. Bitte eine Zahl zwischen 2 und 255 eingeben.");
                return Err(ExitCode::Failure);
            }
        };
    }

    let addr = IpAddress::from(Ipv4Addr::new(192, 168, 0, last_digit));
    let server_ip = IpAddress::from(Ipv4Addr::new(192, 168, 0, 1));

    if fs::write("/dev/net/ip", IpCidr::new(addr, 24).to_string().as_bytes()).is_err() {
        println!("Fehler: IP-Adresse konnte nicht gesetzt werden.");
        return Err(ExitCode::Failure);
    }
    fs::write("/dev/net/gw", b"192.168.0.1").ok(); // Gateway auf Server-IP ist eine gängige Konfiguration.
    fs::write("/ini/dns", b"192.168.0.3").is_ok(); // DNS wird nicht benötigt.

    let server_port = 1234;

    println!("Connecting to TCP server on {}:{}", server_ip, server_port);

    let mut client = UdpSocket::new();
    let mut client2 = UdpSocket::new();

    if let Err(e) = client.connect(server_ip, server_port) {
        println!("1 Failed to connect to server: {:?}", e);
        return Err(ExitCode::Failure);
    }
    else {
        if let Err(e) = client2.connect(server_ip, server_port) {
        println!("2 Failed to connect to server: {:?}", e);
        // return Err(ExitCode::Failure);
    }
    }

    println!("Connected to server.");

    let mut count = 0;
    let mut count2 = 0;

    loop {
        // Sende eine Nachricht an den Server
        let mut status_buf = [0];
        client.read(&mut status_buf).ok();

        if client.poll(IO::Write) {
            let message = format!("1 Hello from client {}! count: {}", last_digit, count);
            if client.write(message.as_bytes()).is_err() {
                println!("Failed to send message to server");
                // break;
            }
            count += 1;
        }

        if client2.poll(IO::Write) {
            let message = format!("2 Hello from client {}! count: {}", last_digit, count2);
            if client2.write(message.as_bytes()).is_err() {
                println!("Failed to send message to server");
                // break;
            }
            count2 += 1;
        }

        // Empfange eine Antwort vom Server
        while client.poll(IO::Read) {
            println!("Client 1 received data from server");
            let mut buf = [0u8; 1024];
            if let Ok(size) = client.read(&mut buf) {
                let msg = &buf[..size];
                println!(
                    "Server sent: {:?}",
                    core::str::from_utf8(msg).unwrap_or("???")
                );
            }
        }
        while client2.poll(IO::Read) {
            println!("Client 2 received data from server");
            let mut buf = [0u8; 1024];
            if let Ok(size) = client2.read(&mut buf) {
                let msg = &buf[..size];
                println!(
                    "Server sent: {:?}",
                    core::str::from_utf8(msg).unwrap_or("???")
                );
            }
        }
    }
}

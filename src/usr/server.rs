use crate::api::fs;
use crate::api::fs::{FileIO, IO};
use crate::api::process::ExitCode;
use crate::sys;
use crate::sys::net::socket::tcp::TcpSocket;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::net::Ipv4Addr;
use smoltcp::wire::{IpAddress, IpCidr};

extern crate alloc;

fn handle_single_client_connection() -> Result<(), String> {
    run_server(1234);

    println!("Neuer Socket wird erstellt und auf Port 1234 gelauscht...");
    let mut connection_socket = TcpSocket::new();

    if let Err(_e) = connection_socket.listen(1234) {
        return Err("Fehler beim Lauschen auf Port 1234".to_string());
    }
    println!("Server lauscht auf Port 1234...");

    match connection_socket.accept() {
        Ok(client_ip) => {
            println!("Verbindung akzeptiert von: {:?}", client_ip);

            // Sende eine Nachricht an den Client
            let response = "Hallo vom Server!\n";
            if let Err(_e) = connection_socket.write(response.as_bytes()) {
                // Optional: Gib mehr Fehlerdetails aus
                // println!("Fehler beim Senden der Daten an {:?}: {:?}", client_ip, e);
                connection_socket.close(); // Wichtig: Socket auch im Fehlerfall schließen
                return Err(format!(
                    "Fehler beim Senden der Daten an Client {:?}",
                    client_ip
                ));
            }
            println!("Antwort an Client {:?} gesendet.", client_ip);

            // Schließe die Verbindung zu diesem spezifischen Client
            connection_socket.close();
            println!("Verbindung mit {:?} geschlossen.", client_ip);
            Ok(()) // Alles gut für diesen Client
        }
        Err(_e) => {
            connection_socket.close();
            Err("Fehler beim Akzeptieren der Verbindung.".to_string())
        }
    }
}

struct Client {
    socket: TcpSocket,
    ip: IpAddress,
}

fn run_server(port: u16) {
    let mut listener = TcpSocket::new();
    listener.listen(port).unwrap();

    let mut clients: Vec<TcpSocket> = Vec::new();

    let mut count = 0;

    loop {
        // Check for new connection
        if listener.poll(IO::Read) {
            if let Ok(remote_ip) = listener.accept() {
                println!("New connection from {:?}", remote_ip);

                let connected_socket = core::mem::replace(&mut listener, TcpSocket::new());
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

    loop {
        match handle_single_client_connection() {
            Ok(_) => {
                println!("Client erfolgreich bedient. Bereit für den nächsten Client.");
            }
            Err(e_msg) => {
                println!(
                    "Ein Fehler ist bei der Client-Behandlung aufgetreten: {}",
                    e_msg
                );

                println!("Versuche erneut, auf einen Client zu warten...");
            }
        }
        println!("----------------------------------------------------");
    }
}

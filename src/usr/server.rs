use alloc::string::{String, ToString};
use alloc::format;
use core::net::Ipv4Addr;
use smoltcp::wire::{IpAddress, IpCidr};
use crate::api::fs;
use crate::api::fs::FileIO;
use crate::api::process::ExitCode;
use crate::sys::net::socket::tcp::TcpSocket;



extern crate alloc;


fn handle_single_client_connection() -> Result<(), String> {
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
                return Err(format!("Fehler beim Senden der Daten an Client {:?}", client_ip));
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
                println!("Ein Fehler ist bei der Client-Behandlung aufgetreten: {}", e_msg);

                println!("Versuche erneut, auf einen Client zu warten...");
            }
        }
        println!("----------------------------------------------------");
    }

}
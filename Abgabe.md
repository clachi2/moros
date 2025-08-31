Guten Abend Herr Schöttner,

hier einmal unsere Abgabe für das Modul MA-Seminar-Systemsoftware-SS25.
Von Rafael Reip und Simon Krämer.
Da wir das Ganze ja zusammen gemacht haben, hier noch einmal die Übersicht, wer welche Teile vom Code und von der Ausarbeitung gemacht hat: (Die Aufteilung ist quasi so, wie im Vortrag)

## Code:
* Erweiterungen an MOROS:
  * src/sys/devices/kprint.rs, src/sys/devices/mod.rs, src/sys/devices/serial.rs: Rafael
  * src/sys/mouse.rs: Rafael
  * src/sys/vga/framebuffer.rs: Simon(String-Ausgabe), Rafael (Rest)
* Spiel:
  * src/usr/game/tanks/*: Rafael (alles, außer:), Simon (Implementierung der Serialize/Deserialize Traits fürs Netzwerk später)
  * src/usr/game/main.rs: Simon (alles, außer:), Rafael (User-input Handling)
  * src/usr/game/network.rs: Simon

## Ausarbeitung:
* Abstract, 1 Einleitung, 2 HINTERGRUND: MOROS, 7 FAZIT UND AUSBLICK; Rafael und Simon zusammen
* 3 ERWEITERUNGEN FÜR MOROS, 4 AUFBAU DES SPIELS: Rafael
* 5 VERTEILTES SPIEL VIA NETZWERK, 6 ANALYSE UND BEWERTUNG: Simon
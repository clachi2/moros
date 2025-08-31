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

## Anleitung zum Kompilieren und Ausführen:

Wichtig: beim ersten Kompilieren und starten muss in MOROS ein neuer Account angelegt, und danach MOROS neu gestartet werden.

### Nur eine Moros-Instanz mit lokalem Spiel:

 ```bash
 make image output=video keyboard=qwertz && make qemu output=video nic=rtl8139
 ```

### Mehrere Moros-Instanzen mit Netzwerkspiel: (ging bei uns nur auf macos)

1. VDE-Switch starten:

 ```bash
vde_switch -s /tmp/moros_switch -m 666
 ```

2. Moros-Instanzen starten:

 ```bash
bash build_and_start_three.sh
 ```

oder

```bash
make image output=video keyboard=qwertz
cp disk.img disk-1.img
cp disk.img disk-2.img
cp disk.img disk-3.img

qemu-system-x86_64 -m 32 -smp 2 -drive file=disk.img,format=raw \
  -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 \
  -netdev vde,id=net0,sock=/tmp/moros_switch \
  -device e1000,netdev=net0,mac=52:54:00:12:34:56 \
  -serial stdio \
  -cpu core2duo &

qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-1.img,format=raw \
  -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 \
  -netdev vde,id=net0,sock=/tmp/moros_switch \
  -device e1000,netdev=net0,mac=52:54:00:12:34:57 \
  -serial stdio \
  -cpu core2duo &

qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-2.img,format=raw \
  -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 \
  -netdev vde,id=net0,sock=/tmp/moros_switch \
  -device e1000,netdev=net0,mac=52:54:00:12:34:58 \
  -serial stdio \
  -cpu core2duo &

wait

 ```


#!/bin/bash

make image output=video keyboard=qwertz

qemu-system-x86_64 -m 32 -smp 2 -drive file=disk.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev socket,id=net0,listen=:12345 -device rtl8139,netdev=net0 -cpu core2duo


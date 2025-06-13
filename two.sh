#!/bin/bash

cp disk.img disk-2.img

#qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-2.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev user,id=e0,hostfwd=tcp::8082-:80 -device rtl8139,netdev=e0, -serial stdio -cpu core2duo
qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-2.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev socket,id=net0,connect=127.0.0.1:12345 -device rtl8139,netdev=net0 -cpu core2duo

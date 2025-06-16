#!/bin/bash

make image output=video keyboard=qwertz
cp disk.img disk-1.img
cp disk.img disk-2.img

#qemu-system-x86_64 -m 32 -smp 2 -drive file=disk.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev socket,id=net0,listen=:12345 -device rtl8139,netdev=net0 -cpu core2duo &
#qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-1.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev socket,id=net0,connect=127.0.0.1:12345 -device rtl8139,netdev=net0 -cpu core2duo &
#qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-2.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -netdev socket,id=net0,connect=127.0.0.1:12345 -device rtl8139,netdev=net0 -cpu core2duo &

qemu-system-x86_64 -m 32 -smp 2 -drive file=disk.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -device e1000,netdev=n1,mac=52:54:00:12:34:56 -netdev socket,id=n1,mcast=230.0.0.1:1234 -cpu core2duo &
qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-1.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -device e1000,netdev=n2,mac=52:54:00:12:34:57 -netdev socket,id=n2,mcast=230.0.0.1:1234 -cpu core2duo &
qemu-system-x86_64 -m 32 -smp 2 -drive file=disk-2.img,format=raw -audiodev coreaudio,id=a0 -machine pcspk-audiodev=a0 -device e1000,netdev=n3,mac=52:54:00:12:34:58 -netdev socket,id=n3,mcast=230.0.0.1:1234 -cpu core2duo &

wait
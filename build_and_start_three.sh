#!/bin/bash

# needs to have vde_switch running with:
# vde_switch -s /tmp/moros_switch -m 666

make image output=video keyboard=qwertz
cp disk.img disk-1.img
cp disk.img disk-2.img

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
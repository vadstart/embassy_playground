#!/bin/sh

sudo picotool load -u -v -x -t elf "$1" || exit 1

sleep 1

PORT=$(ls /dev/tty.usbmodem* | head -n 1)

screen "$PORT" 115200

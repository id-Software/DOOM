#!/bin/sh

make clean
make
(cd linux; DISPLAY=:1 ./linuxxdoom)

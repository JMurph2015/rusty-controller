#!/bin/bash
cargo build --release;
ssh pi@192.168.0.100 "rm -rf ~/rusty_controller";
scp -r . pi@192.168.0.100:~/rusty_controller;
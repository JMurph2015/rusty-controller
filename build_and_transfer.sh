#!/bin/bash
cargo build --release --target=armv7-unknown-linux-gnueabihf;
ssh pi@192.168.0.100 "rm -rf ~/rusty_controller";
scp -r ./target/armv7-unknown-linux-gnueabihf/release pi@192.168.0.100:~/rusty_controller;
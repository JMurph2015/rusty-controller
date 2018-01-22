# Rusty Controller
An LED controller for Irradiance written in Rust.  Intended for use on the Raspberry Pi with Neopixel RGB LEDs.

# Installation
Simply clone the repo onto either your Raspberry Pi or your cross-compile capable desktop or laptop.  Then compile the library with ` cargo build --release ` and transfer the generated binaries (the whole folder of ` target/armv7-unknown-gnueabihf/release `) to your Raspberry Pi.  Then execute it with ` sudo ./rusty_controller ` from that folder.
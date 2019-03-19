#![allow(non_snake_case)]

extern crate chrono;
extern crate colored;
extern crate fern;
#[macro_use]
extern crate log;
extern crate mio;
extern crate pnet;
extern crate rs_ws281x;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use mio::net::UdpSocket;
use mio::{Events, Poll, PollOpt, Ready, Token};
use pnet::datalink;
use rs_ws281x::RawColor;

mod config;
use config::{create_handler, ControllerConfig};
mod packets;
use packets::{ConfigPacket, StartupMessage, StripConfigPacket};
mod setup;
use setup::{setup_logging, setup_server_connection};

use std::{
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    panic, thread,
    time::Duration,
};

pub const UDP_MAX_PACKET_SIZE: u32 = 65507 as u32;

#[derive(Debug, Clone)]
pub struct Handler(pub rs_ws281x::Controller);

impl Drop for Handler {
    fn drop(&mut self) {
        for channel_num in self.0.channels().iter_mut() {
            let mut ledstrip = self.0.leds_mut(*channel_num);
            for mut led in ledstrip.iter_mut() {
                *led = [0x00, 0x00, 0x00, 0x00];
            }
        }
        self.0.render().expect("Failed to render");
    }
}

fn main() {
    setup_logging().unwrap();

    info!("Starting rusty_controller...");

    let config: ControllerConfig = serde_json::from_str(
        &fs::read_to_string("/etc/rusty_controller/config.json")
            .expect("Failed to read config file"),
    ).expect("Failed to deserialize config.");

    // Initialize the Handler
    let mut led_handler = Handler(create_handler(config.clone()));

    // Initialize variables for server connection
    const MAIN_SOCKET: Token = Token(0);
    let addr = Ipv4Addr::new(0, 0, 0, 0);
    let bind_addr = SocketAddr::from((addr, config.port));
    let main_socket = UdpSocket::bind(&bind_addr).expect("Failed to bind socket");
    main_socket
        .set_broadcast(true)
        .expect("Failed to set broadcast");

    // Set up poll struct
    let poll = Poll::new().expect("Failed to make poll");
    poll.register(
        &main_socket,
        MAIN_SOCKET,
        Ready::readable(),
        PollOpt::edge(),
    ).expect("Failed to register socket");

    // Set up event queue
    let mut events = Events::with_capacity(128);

    info!("Initialized the main UDP socket.");

    for i in 0..4 {
        if i % 2 == 0 {
            set_all_rgb(led_handler.0.leds_mut(0), 0x55, 0x02, 0x01);
        } else {
            set_all_rgb(led_handler.0.leds_mut(0), 0x01, 0x55, 0x02);
        }
        led_handler.0.render().expect("Failed to render");
        thread::sleep(Duration::from_millis(250));
    }
    set_all_rgb(led_handler.0.leds_mut(0), 0x00, 0x00, 0x00);
    led_handler.0.render().expect("Failed to render");

    // Setup server connection
    setup_server_connection(&poll, &main_socket, config.clone());

    // Setup polling variables for LED data
    let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];
    let default_data = (0usize, SocketAddr::from(([127, 0, 0, 1], 8080)));

    let mut elapsed: Duration = Duration::from_millis(0);
    let timeout: Duration = Duration::from_secs(30);
    let reset_duration = Duration::from_millis(0);
    let poll_rate = Duration::from_millis(5);
    let polling_rate = Some(poll_rate);

    // Begin main loop
    'main_loop: loop {
        poll.poll(&mut events, polling_rate)
            .expect("Failed to poll");
        for event in events.iter() {
            match event.token() {
                MAIN_SOCKET => {
                    let (received, _) = match main_socket.recv_from(&mut buf) {
                        Ok(n) => {
                            elapsed = reset_duration;
                            n
                        }
                        Err(_e) => default_data,
                    };
                    if received != 0usize {
                        parse_and_update(&mut led_handler.0, &buf[..received]);
                        led_handler.0.render().expect("Render failed...");
                    }
                }
                _ => {}
            }
        }
        elapsed = match elapsed.checked_add(poll_rate) {
            None => timeout,
            Some(x) => x,
        };
        if elapsed > timeout {
            setup_server_connection(&poll, &main_socket, config.clone());
            elapsed = reset_duration;
        }
    }
}

fn parse_and_update(handler: &mut rs_ws281x::Controller, raw_data: &[u8]) {
    let mut offset = 0;
    for channel_num in handler.channels().iter() {
        let mut ledstrip = handler.leds_mut(channel_num.clone());
        for i in 0..ledstrip.len() {
            if (3 * i + 2) + offset < raw_data.len() {
                ledstrip[i] = [
                    0xFF,
                    raw_data[3 * i + 0 + offset],
                    raw_data[3 * i + 1 + offset],
                    raw_data[3 * i + 2 + offset],
                ]
            }
        }
        offset += ledstrip.len();
    }
}

fn set_all_rgb(ledstrip: &mut [RawColor], r: u8, g: u8, b: u8) {
    for i in 0..ledstrip.len() {
        ledstrip[i] = [0xFF, r, g, b];
    }
}

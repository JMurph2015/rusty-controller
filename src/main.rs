#![allow(non_snake_case)]
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate mio;
use mio::net::UdpSocket;
use mio::{Events, Ready, Poll, PollOpt, Token};

extern crate ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

extern crate rs_ws281x;
use rs_ws281x::{RawColor};

mod handle_json;
use handle_json::{StartupMessage, ConfigPacket, StripConfigPacket};

mod config;
use config::{ControllerConfig, create_handler};

use std::{
	thread,
	fs,
	process::{Command},
	time::{Duration},
	net::{SocketAddr, Ipv4Addr}
};

pub const UDP_MAX_PACKET_SIZE: u32 = 65507 as u32;

fn main() {
    println!("Starting rusty_controller...");

	let config: ControllerConfig = serde_json::from_str(
		&fs::read_to_string("/led_config.json")
			.expect("Failed to read config file")
	).expect("Failed to deserialize config.");

    // Set up a handler for Ctrl-C events
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Initialize the Handler
	let mut led_handler = create_handler(config.clone());

    // Initialize variables for server connection
    const MAIN_SOCKET: Token = Token(0);
    let addr = Ipv4Addr::new(0, 0, 0, 0);
    let bind_addr = SocketAddr::from((addr, config.port));
    let main_socket = UdpSocket::bind(&bind_addr)
        .expect("Failed to bind socket");
    main_socket.set_broadcast(true)
        .expect("Failed to set broadcast");
    
    // Set up poll struct
    let poll = Poll::new().expect("Failed to make poll");
    poll.register(&main_socket, MAIN_SOCKET, Ready::readable(), PollOpt::edge())
        .expect("Failed to register socket");

    // Set up event queue
    let mut events = Events::with_capacity(128);

    println!("Initialized the main UDP socket.");

    for i in 0..4 {
        if i % 2 == 0 {
            set_all_rgb(led_handler.leds_mut(0), 0x55, 0x02, 0x01);
        } else {
            set_all_rgb(led_handler.leds_mut(1), 0x01, 0x55, 0x02);
        }
        led_handler.render().expect("Failed to render");
        thread::sleep(Duration::from_millis(250));
    }
    set_all_rgb(led_handler.leds_mut(0), 0x00, 0x00, 0x00);
    led_handler.render().expect("Failed to render");
    
    // Setup server connection
    setup_server_connection(
        &poll,
        &main_socket,
		config.clone()
    );

    // Setup polling variables for LED data
    let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];
	let default_data = ( 0usize, SocketAddr::from(([127,0,0,1], 8080)));

	let mut elapsed: Duration = Duration::from_millis(0);
	let timeout: Duration = Duration::from_secs(30);
	let reset_duration = Duration::from_millis(0);
	let poll_rate = Duration::from_millis(5);
	let polling_rate = Some(poll_rate);

    // Begin main loop
    'main_loop: loop {
		if !running.load(Ordering::SeqCst){
			break 'main_loop;
		}
		poll.poll( &mut events, polling_rate )
			.expect("Failed to poll");
		for event in events.iter() {
			match event.token() {
				MAIN_SOCKET => {
					let ( received, _ ) = match main_socket.recv_from(&mut buf) {
						Ok( n ) => {
							elapsed = reset_duration;
							n
						},
						Err( _e ) => default_data,
					};
					if received != 0usize {
						parse_and_update( &mut led_handler, &buf[..received]);
                        led_handler.render().expect("Render failed...");
					}
				},
				_ => {

				}
			}
		}
		elapsed = match elapsed.checked_add(poll_rate) {
			None => timeout,
			Some( x ) => x
		};
		if elapsed > timeout {
			setup_server_connection(
				&poll,
				&main_socket,
				config.clone()
			);
			elapsed = reset_duration;
		}
	}
	set_all_rgb(led_handler.leds_mut(0), 0o0, 0o0, 0o0);
    led_handler.render().expect("Failed to render");
}

fn setup_server_connection( 
	poll: &Poll, 
	main_udpsock: &UdpSocket,
	config: ControllerConfig
	) {
	// ifconfig | grep -i "inet " | sed -r 's/.*addr:([^ ]+).*/\1/' | tr '\n' ' '
	let output = Command::new("hostname")
		.arg("-I")
		.output()
		.expect("Failed to execute hostname command.");
	let ip_string = String::from_utf8_lossy( &output.stdout ).split_whitespace().next()
		.expect("Sad times since there were no IP's listed.").to_string();

	println!("IP Address: {}", ip_string);

	let controller_config = ConfigPacket {
		name: config.clone().name,
		ip: ip_string,
		port: config.port as i64,
		numStrips: config.clone().strips.len() as i64,
		numAddrs: config.clone().num_addrs().into(),
		mac: "none".to_string(),
		strips: config.strips.iter().map(|x: _| {
			StripConfigPacket {
				name: x.name.clone(),
				startAddr: x.startAddr.clone() as i64,
				endAddr: x.endAddr.clone() as i64,
				channel: x.channel.clone() as i64,
			}
		}).collect::<Vec<StripConfigPacket>>()
	};

	let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];
	let mut events = Events::with_capacity(128);

	println!("Listening for handshake data...");

	let default_data = ( 0usize, SocketAddr::from(([127,0,0,1], 8080)));

	let (received, mut src_addr) = 'outer: loop {
		poll.poll( &mut events, Some(Duration::from_millis(50)) )
			.expect("Failed to poll");
		for event in events.iter() {
			match event.token() {
				_ => {
					match main_udpsock.recv_from( &mut buf ) {
						Ok( n ) => {
							break 'outer n
						}
						Err( e ) => {
							println!("{}", e);
							break 'outer default_data;
						}
					}
				}
			}
		}
	};
	
	if received != 0usize {
		
		let data = &buf[..received];
		let _json_data: StartupMessage = serde_json::from_slice(data)
			.expect("Failed to parse JSON.");
		println!("Found startup message");
		// TODO Error handling on the json decoding path
		let output = serde_json::to_vec(&controller_config)
			.expect("Failed to render JSON");

		src_addr.set_port(config.setup_port);

		main_udpsock.send_to(&output, &src_addr)
			.expect("Couldn't send data to address");
	}
}

fn parse_and_update( handler: &mut rs_ws281x::Controller, raw_data: &[u8] ) {
	let mut offset = 0;
	for channel_num in handler.channels().iter() {
		let mut ledstrip = handler.leds_mut(channel_num.clone());
		for i in 0..ledstrip.len() {
			if (3*i + 2) + offset < raw_data.len() {
				ledstrip[i] = [
					0xFF,
					raw_data[3*i + 0 + offset], 
					raw_data[3*i + 1 + offset],
					raw_data[3*i + 2 + offset],
				]
			}
		}
		offset += ledstrip.len();
	}
}

fn set_all_rgb( ledstrip: &mut [RawColor], r: u8, g: u8, b: u8) {
    for i in 0..ledstrip.len() {
        ledstrip[i] = [0xFF, r, g, b];
    }
}
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate mio;
use mio::net::UdpSocket;
use mio::{Events, Ready, Poll, PollOpt, Token};

extern crate ctrlc;
use ctrlc::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

extern crate rs_ws281x;
use rs_ws281x::*;

mod constants;
use constants::*;

mod handle_json;
use handle_json::{StartupMessage, ControllerConfig, StripConfig};

use std::net::{SocketAddr, Ipv4Addr, ToSocketAddrs};

use std::thread;
use std::process::Command;
use std::time::Duration;
use std::string::String;
use std::ptr::{null, null_mut};

fn main() {
	println!("Starting rusty_controller...");
	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();
	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");
	let mut ret: ws2811_return_t;
	let mut ledstring = ws2811_t {
		render_wait_time: 0,
		device: null_mut(),
		rpi_hw: null(),
		freq: LED_FREQ_HZ,
		dmanum: LED_DMA,
		channel: [
			ws2811_channel_t {
				gpionum: LED_PIN,
				count: LED_COUNT,
				invert: LED_INVERT,
				brightness: LED_BRIGHTNESS,
				strip_type: WS2811_STRIP_GRB as i32,
				leds: null_mut(),
				gamma: null_mut(),
				wshift: 0o0,
				rshift: 0o0,
				gshift: 0o0,
				bshift: 0o0,
			},
			ws2811_channel_t {
				gpionum: 0,
				count: 0,
				invert: LED_INVERT,
				brightness: 0,
				strip_type: WS2811_STRIP_GRB as i32,
				leds: null_mut(),
				gamma: null_mut(),
				wshift: 0o0,
				rshift: 0o0,
				gshift: 0o0,
				bshift: 0o0,
			}
		]
	};
	let mut ledstring_ptr: *mut ws2811_t = &mut ledstring;
	unsafe {
		ret = ws2811_init(ledstring_ptr);
		if ret != ws2811_return_t::WS2811_SUCCESS {
			println!("ws2811_init failed: {:?}\n", ws2811_get_return_t_str(ret));
			panic!("Init failed");
		}
	}
	

	println!("Initialized the LED handler.");

	const MAIN_SOCKET: Token = Token(0);	
	let addr = Ipv4Addr::new(0, 0, 0, 0);
	let bind_addr = SocketAddr::from((addr, MAIN_PORT));
	let main_socket = UdpSocket::bind(&bind_addr)
		.expect("Failed to bind socket");
	main_socket.set_broadcast(true)
		.expect("Failed to set broadcast");

	let poll = Poll::new().expect("Failed to make poll");
	poll.register(&main_socket, MAIN_SOCKET, Ready::readable(), PollOpt::edge())
		.expect("Failed to register socket");

	let mut events = Events::with_capacity(128);
	

	println!("Initialized the main UDP socket.");

	for i in 0..4 {
		if i % 2 == 0 {
			set_all_rgb(ledstring, 0x55, 0x02, 0x01);
		} else {
			set_all_rgb(ledstring, 0x01, 0x55, 0x02);
		}
		unsafe {
			ws2811_render(ledstring_ptr);
		}
		thread::sleep(Duration::from_millis(250));
	}
	set_all_rgb(ledstring, 0x00, 0x00, 0x00);
	unsafe {
		ws2811_render(ledstring_ptr);
	}
	

	setup_server_connection(
		&poll,
		&main_socket,
		CONTROLLER_NAME.to_string(), 
		LED_PER_ROW, 
		NUM_ROW,  
		MAIN_PORT, 
		SETUP_PORT
	);

	let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];
	let default_data = ( 0usize, SocketAddr::from(([127,0,0,1], 8080)));

	let mut elapsed: Duration = Duration::from_millis(0);
	let timeout: Duration = Duration::from_secs(30);
	let reset_duration = Duration::from_millis(0);
	let poll_rate = Duration::from_millis(5);
	let polling_rate = Some(poll_rate);

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
						parse_and_update( ledstring, &buf[..received]);
						unsafe {
							ret = ws2811_render(ledstring_ptr);
							if ret != ws2811_return_t::WS2811_SUCCESS {
								println!("ws2811_render failed: {:?}", ws2811_get_return_t_str(ret));
								break 'main_loop;
							}
						}
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
				CONTROLLER_NAME.to_string(), 
				LED_PER_ROW, 
				NUM_ROW,  
				MAIN_PORT, 
				SETUP_PORT
			);
			elapsed = reset_duration;
		}
	}
	set_all_rgb(ledstring, 0o0, 0o0, 0o0);
	unsafe {
		ws2811_render(ledstring_ptr);
		ws2811_fini(ledstring_ptr);
	}
}

fn setup_server_connection( 
	poll: &Poll, 
	main_udpsock: &UdpSocket,
	name: String, 
	led_per_row: i64, 
	num_rows: i64, 
	main_port: u16, 
	setup_port: u16
	) {
	let output = Command::new("hostname")
		.arg("-I")
		.output()
		.expect("Failed to execute hostname command.");
	let ip_string = String::from_utf8_lossy( &output.stdout ).split_whitespace().next()
		.expect("Sad times since there were no IP's listed.").to_string();

	println!("IP Address: {}", ip_string);

	let controller_config = ControllerConfig {
		name: name,
		ip: ip_string,
		port: main_port as i64,
		numStrips: num_rows,
		numAddrs: led_per_row*num_rows,
		mac: "none".to_string(),
		strips: &[
			StripConfig {
				name: "1".to_string(),
				startAddr: 1,
				endAddr: 30,
				channel: 1
			}
		]
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
		let json_data: StartupMessage = serde_json::from_slice(data)
			.expect("Failed to parse JSON.");
		println!("Found startup message");
		// TODO Error handling on the json decoding path
		let output = serde_json::to_vec(&controller_config)
			.expect("Failed to render JSON");

		src_addr.set_port(setup_port);

		main_udpsock.send_to(&output, &src_addr)
			.expect("Couldn't send data to address");
	}
}

fn parse_and_update( ledstrip: ws2811_t, raw_data: &[u8] ) {
	unsafe {
		for i in 0..ledstrip.channel[0].count {
			if (3*i) < raw_data.len() as i32 {
				*ledstrip.channel[0].leds.offset(i as isize) = compose_color(raw_data[(3*i) as usize], raw_data[(3*i + 1) as usize], raw_data[(3*i+2) as usize]);
			}
		}
	}
	
}

fn set_all_rgb( ledstrip: ws2811_t, r: u8, g: u8, b: u8) {
	unsafe {
		for i in 0..ledstrip.channel[0].count {
			*ledstrip.channel[0].leds.offset(i as isize) = compose_color(r,g,b);
		}
	}
}

fn compose_color( r: u8, g: u8, b: u8) -> u32 {
	return 0xFF000000 as u32 + (r as u32)*2_u32.pow(16) + (g as u32)*2_u32.pow(8) + b as u32;
}
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate ws281x;

use ws281x::Handle;

use serde_json::from_slice;

mod constants;
use constants::*;

mod handle_json;
use handle_json::StartupMessage;
use handle_json::ControllerConfig;
use handle_json::StripConfig;

use std::thread;
use std::time::Duration;
use std::net::UdpSocket;
use std::process::Command;
use std::string::String;
use std::option::Option;

fn main() {
	println!("Starting rusty_controller...");
    let mut handler = ws281x::handle::new()
		.dma(LED_DMA)
		.channel(0, ws281x::channel::new()
			.pin(LED_PIN)
			.count(LED_COUNT as usize)
			.brightness(LED_BRIGHTNESS as i32)
			.build().unwrap())
		.build().unwrap();

	println!("Initialized the LED handler.");

	let mut master_buf = [0x0; UDP_MAX_PACKET_SIZE as usize];

	
	let mut main_udpsock = UdpSocket::bind(format!("127.0.0.1:{}", MAIN_PORT))
		.expect("Failed to connect to the socket");

	println!("Initialized the main UDP socket.");

	for i in 0..10 {
		if i % 2 == 0 {
			set_all_rgb(&mut handler, 0xff, 0x02, 0x01);
		} else {
			set_all_rgb(&mut handler, 0x01, 0xff, 0x02);
		}
		thread::sleep(Duration::from_millis(250));
	}
	set_all_rgb(&mut handler, 0x00, 0x00, 0x00);

	setup_server_connection(
		CONTROLLER_NAME.to_string(), 
		LED_PER_ROW, 
		NUM_ROW, 
		&mut main_udpsock, 
		MAIN_PORT, 
		SETUP_PORT
	);

	loop {
		let received = match main_udpsock.recv(&mut master_buf) {
			Ok( received ) => received,
			Err( e ) => 1usize
		};
		if received != 1usize {
			parse_and_update(&mut handler, &master_buf);
		}
    }
	
}

fn setup_server_connection(name: String, led_per_row: i64, num_rows: i64, main_udpsock: &UdpSocket, main_port: u32, setup_port: u32) {
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

	println!("constructed controller config");

	main_udpsock.set_read_timeout(Option::Some(Duration::from_secs(60)))
		.expect("Failed to set timeout.");

	let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];

	let received = match main_udpsock.recv(&mut buf) {
		Ok( received ) => received,
		Err( e ) => 1usize,
	};

	println!("Received some sort of data");
	
	if received != 1usize {
		let data = &buf[0..received];
		let json_data: StartupMessage = serde_json::from_slice(data)
			.expect("Failed to parse JSON.");
		println!("Found startup message");
		// TODO Error handling on the json decoding path
		let output = serde_json::to_vec(&controller_config)
			.expect("Failed to render JSON");

		main_udpsock.send_to(&output, format!("{}:{}", json_data.ip, setup_port))
			.expect("Couldn't send data to address");
	}
}

fn parse_and_update( ledstrip: &mut Handle, raw_data: &[u8] ) {
	for (i, led) in ledstrip.channel_mut(0).leds_mut().iter_mut().enumerate() {
		if i >= 2 {
			*led = (raw_data[(3*i)-2] as u32)*2_u32.pow(16) + (raw_data[(3*i) - 1] as u32)*2_u32.pow(8) + (raw_data[3*i] as u32);
		}
	}
	ledstrip.render().unwrap();
	ledstrip.wait().unwrap();
}

fn set_all_rgb( ledstrip: &mut Handle, r: u8, g: u8, b: u8) {
	for (i, led) in ledstrip.channel_mut(0).leds_mut().iter_mut().enumerate() {
		*led = (r as u32)*2_u32.pow(16) + (g as u32)*2_u32.pow(8) + b as u32;
	}
	ledstrip.render().unwrap();
	ledstrip.wait().unwrap();
}
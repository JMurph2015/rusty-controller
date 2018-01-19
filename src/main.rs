extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate ws281x;

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
    let mut handler = ws281x::handle::new()
		.dma(LED_DMA)
		.channel(0, ws281x::channel::new()
			.pin(LED_PIN)
			.count(LED_COUNT as usize)
			.brightness(LED_BRIGHTNESS as i32)
			.build().unwrap())
		.build().unwrap();

	let mut master_buf = [0x0; UDP_MAX_PACKET_SIZE as usize];

	
	let main_udpsock = UdpSocket::bind(format!("127.0.0.1:{}", MAIN_PORT))
		.expect("Failed to connect to the socket");

	setup_server_connection(
		CONTROLLER_NAME, 
		LED_PER_ROW, 
		NUM_ROW, 
		&mut main_udpsock, 
		MAIN_PORT, 
		SETUP_PORT, 
		&mut master_buf
	);

	let mut check = 0;

	loop {
		for (i, led) in handler.channel_mut(0).leds_mut().iter_mut().enumerate() {
			if i % 2 == check {
				*led = 0
			}
			else {
				*led = 0xffffff;
			}
		}

		handler.render().unwrap();
		handler.wait().unwrap();

		thread::sleep(Duration::from_millis(500));
		check = if check == 0 { 1 } else { 0 };
    }
	
}

fn setup_server_connection(name: String, led_per_row: i64, num_rows: i64, main_udpsock: &mut UdpSocket, main_port: u32, setup_port: u32, buf: &mut [u8]) {
	let output = Command::new("hostname")
		.arg("-I")
		.output()
		.expect("Failed to execute hostname command.");
	let data_string: &[str] = String::from_utf8_lossy( &output.stdout ).split_whitespace().collect();
	let ip_string: String = data_string[0] as String;


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

	main_udpsock.set_read_timeout(Option::Some(Duration::from_secs(60)));

	let received = match main_udpsock.recv(&mut buf) {
		Ok( received ) => received,
		Err( e ) => 1usize,
	};
	
	if received != 1usize {
		let data: StartupMessage = serde_json::from_slice(&buf[0..received]).unwrap()
			.expect("Failed to parse json.");
		// TODO Error handling on the json decoding path
		let output = serde_json::to_vec(controller_config);

		main_udpsock.send_to(output, format!("{}:{}", data.ip, setup_port))
			.expect("Couldn't send data to address");
	}
}


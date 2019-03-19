use super::*;

use fern::colors::{Color, ColoredLevelConfig};

use std::ops::Deref;

pub fn setup_server_connection(poll: &Poll, main_udpsock: &UdpSocket, config: ControllerConfig) {
    let ip_addr =
        find_matching_ip(config.subnet, config.netmask).expect("Failed to find matching subnet");

    let ip_string = format!(
        "{}.{}.{}.{}",
        ip_addr[0], ip_addr[1], ip_addr[2], ip_addr[3]
    );

    info!("IP Address: {}", ip_string);

    let controller_config = ConfigPacket {
        name: config.clone().name,
        ip: ip_string,
        port: config.port as i64,
        numStrips: config.clone().strips.len() as i64,
        numAddrs: config.clone().num_addrs().into(),
        mac: "none".to_string(),
        strips: config
            .strips
            .iter()
            .map(|x: _| StripConfigPacket {
                name: x.name.clone(),
                startAddr: x.startAddr.clone() as i64,
                endAddr: x.endAddr.clone() as i64,
                channel: x.channel.clone() as i64,
            }).collect::<Vec<StripConfigPacket>>(),
    };

    let mut buf = [0x0; UDP_MAX_PACKET_SIZE as usize];
    let mut events = Events::with_capacity(128);

    info!("Listening for handshake data...");

    let default_data = (0usize, SocketAddr::from(([127, 0, 0, 1], 8080)));

    let (received, mut src_addr) = 'outer: loop {
        poll.poll(&mut events, Some(Duration::from_millis(50)))
            .expect("Failed to poll");
        for event in events.iter() {
            match event.token() {
                _ => match main_udpsock.recv_from(&mut buf) {
                    Ok(n) => break 'outer n,
                    Err(e) => {
                        info!("{}", e);
                        break 'outer default_data;
                    }
                },
            }
        }
    };

    if received != 0usize {
        let data = &buf[..received];
        let _json_data: StartupMessage =
            serde_json::from_slice(data).expect("Failed to parse JSON.");
        info!("Found startup message");
        // TODO Error handling on the json decoding path
        let output = serde_json::to_vec(&controller_config).expect("Failed to render JSON");

        src_addr.set_port(config.setup_port);

        main_udpsock
            .send_to(&output, &src_addr)
            .expect("Couldn't send data to address");
    }
}

pub fn find_matching_ip(subnet: [u8; 4], netmask: [u8; 4]) -> Result<[u8; 4], String> {
    let ifaces = datalink::interfaces();
    let ips = ifaces
        .iter()
        .map(|x: _| x.ips.iter())
        .flatten()
        .map(|x: _| x.ip())
        .filter(|x: _| x.is_ipv4())
        .map(|x: _| match x {
            IpAddr::V4(s) => s,
            IpAddr::V6(_s) => Ipv4Addr::new(0, 0, 0, 0),
        }).map(|x: _| x.octets())
        .filter(|x: _| {
            x.iter()
                .zip(netmask.iter())
                .map(|y: (_, _)| y.0.clone() & y.1.clone())
                .zip(subnet.iter())
                .map(|y: (_, _)| y.0.clone() == y.1.clone())
                .fold(true, |y: _, z: _| y && z)
        }).collect::<Vec<_>>();
    if ips.len() > 0 {
        return Ok(ips[0]);
    } else {
        return Err("No matching IP's found".into());
    }
}

pub fn setup_logging() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .trace(Color::Cyan)
        .debug(Color::Blue)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}]{}[{}] {}",
                colors.color(record.level()),
                chrono::Utc::now().format("[%Y-%m-%d][%H:%M:%S%.3f]"),
                record.target(),
                message
            ))
        }).level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("/var/log/rusty_controller.log")?)
        .apply()?;

    panic::set_hook(Box::new(|panic_info| {
        let (filename, line) = panic_info
            .location()
            .map(|loc| (loc.file(), loc.line()))
            .unwrap_or(("<unknown>", 0));

        let cause = panic_info
            .payload()
            .downcast_ref::<String>()
            .map(String::deref);

        let cause = cause.unwrap_or_else(|| {
            panic_info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| *s)
                .unwrap_or("<cause unknown>")
        });

        error!("A panic occurred at {}:{}: {}", filename, line, cause);
    }));
    Ok(())
}

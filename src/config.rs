use rs_ws281x;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub name: String,
    pub port: u16,
    pub setup_port: u16,
    pub dma: i32,
    pub freq: u32,
    pub channels: Vec<ChannelConfig>,
    pub strips: Vec<StripConfig>,
}

impl ControllerConfig {
    pub fn num_addrs(&self) -> i32 {
        return self.channels.iter().map(|x: _| x.count).sum();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub num: usize,
    pub pin: i32,
    pub count: i32,
    pub invert: bool,
    pub brightness: u8,
    pub strip_type: rs_ws281x::StripType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripConfig {
    pub name: String,
    pub startAddr: usize,
    pub endAddr: usize,
    pub channel: usize,
}

pub fn create_handler(config: ControllerConfig) -> rs_ws281x::Controller {
    let builder = &mut rs_ws281x::ControllerBuilder::new();
    builder
        .freq(config.freq)
        .dma(config.dma);
    for channel_config in config.channels.iter() {
        builder.channel(
            channel_config.num,
            rs_ws281x::ChannelBuilder::new()
                .pin(channel_config.pin)
                .count(channel_config.count)
                .invert(channel_config.invert)
                .brightness(channel_config.brightness)
                .strip_type(channel_config.strip_type)
                .build()
        );
    }
    return builder.build().expect("Failed to build handler");
}
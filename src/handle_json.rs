#[derive(Serialize, Deserialize)]
pub struct StartupMessage {
    pub ip: String,
    pub mac: String,
    pub msg_type: String
}

#[derive(Serialize, Deserialize)]
pub struct ControllerConfig<'a> {
    pub name: String,
    pub ip: String,
    pub port: i64,
    pub mac: String,
    pub numStrips: i64,
    pub numAddrs: i64,
    pub strips: &'a[StripConfig]
}

#[derive(Serialize, Deserialize)]
pub struct StripConfig {
    pub name: String,
    pub startAddr: i64,
    pub endAddr: i64,
    pub channel: i64
}
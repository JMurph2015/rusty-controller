#[derive(Serialize, Deserialize)]
pub struct StartupMessage {
    pub ip: String,
    pub mac: String,
    pub msg_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigPacket {
    pub name: String,
    pub ip: String,
    pub port: i64,
    pub mac: String,
    pub numStrips: i64,
    pub numAddrs: i64,
    pub strips: Vec<StripConfigPacket>,
}

#[derive(Serialize, Deserialize)]
pub struct StripConfigPacket {
    pub name: String,
    pub startAddr: i64,
    pub endAddr: i64,
    pub channel: i64,
}

use std::{io::ErrorKind, time::Duration};

use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize,
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, time::timeout};

const PACKET: &[u8; 29] = &[
    0x00, 0x83, // Header
    0x00, 0x19, // Length
    0x00, 0x00, 0x00, 0x00, 0x00, // Padding
    0x3F, 0x73, 0x74, 0x61, 0x74, 0x75, 0x73, 0x26, 0x66, 0x6F, 0x72, 0x6D, 0x61, 0x74, 0x3D, 0x6A,
    0x73, 0x6F, 0x6E, // ?status&format=json
    0x00, // Footer
];

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerInfo {
    pub version: String,
    #[serde(deserialize_with = "bool_from_int")]
    pub respawn: bool,
    #[serde(deserialize_with = "bool_from_int")]
    pub enter: bool,
    #[serde(deserialize_with = "bool_from_int")]
    pub ai: bool,
    pub host: Option<String>,
    pub round_id: Option<String>,
    pub players: u32,
    pub revision: String,
    pub revision_date: String,
    #[serde(deserialize_with = "bool_from_int")]
    pub hub: bool,
    pub identifier: String,
    pub admins: u32,
    pub gamestate: u32,
    pub map_name: String,
    pub security_level: String,
    pub round_duration: f32,
    pub time_dilation_current: f32,
    pub time_dilation_avg: f32,
    pub time_dilation_avg_slow: f32,
    pub time_dilation_avg_fast: f32,
    pub soft_popcap: u32,
    pub hard_popcap: u32,
    pub extreme_popcap: u32,
    pub popcap: Option<u32>,
    #[serde(deserialize_with = "bool_from_int_opt", default)]
    pub bunkered: Option<bool>,
    #[serde(deserialize_with = "bool_from_int_opt", default)]
    pub interviews: Option<bool>,
    pub shuttle_mode: Option<String>,
    pub shuttle_timer: Option<u32>,
    pub active_players: Option<u32>,
    pub public_address: Option<String>,
}

pub async fn query_server(server: &str) -> std::io::Result<ServerInfo> {
    let mut stream = tokio::net::TcpStream::connect(server).await?;
    timeout(Duration::from_secs_f32(0.75), stream.write_all(PACKET)).await??;
    let mut resp_header = [0u8; 2];
    timeout(Duration::from_secs_f32(0.75),stream.read_exact(&mut resp_header)).await??;
    if resp_header != [0x00, 0x83] {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "invalid header",
        ));
    }
    let mut resp_len = [0u8; 2];
    stream.read_exact(&mut resp_len).await?;
    let resp_len = u16::from_be_bytes(resp_len);
    let mut resp_type = [0u8; 1];
    stream.read_exact(&mut resp_type).await?;
    if resp_type != [0x06] {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "invalid type"));
    }
    let mut resp_data = vec![0u8; resp_len as usize - 2];
    stream.read_exact(&mut resp_data).await?;
    let mut resp_footer = [0u8; 1];
    stream.read_exact(&mut resp_footer).await?;
    if resp_footer != [0x00] {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "invalid footer",
        ));
    }
    let json = String::from_utf8(resp_data).unwrap();
    serde_json::from_str(json.trim()).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
}

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(de::Error::invalid_value(
            Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}

fn bool_from_int_opt<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(deserializer)? {
        Some(0) => Ok(Some(false)),
        Some(1) => Ok(Some(true)),
        None => Ok(None),
        Some(other) => Err(de::Error::invalid_value(
            Unexpected::Unsigned(other as u64),
            &"zero, one, or null",
        )),
    }
}

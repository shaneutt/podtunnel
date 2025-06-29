use crate::helpers::ObjectReference;

use std::net::Ipv4Addr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub enum WireguardPeer {
    Config(WireguardPeerConfig),
    Pod(ObjectReference),
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct WireguardPeerConfig {
    pub public_key: String,

    pub endpoint_address: Ipv4Addr,

    #[serde(default = "crate::wireguard::default_wireguard_listen_port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_port: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tunnel_address: Option<Ipv4Addr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tunnel_address_prefix: Option<u8>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ips: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistent_keepalive: Option<i32>,
}

impl WireguardPeerConfig {
    pub fn endpoint(&self) -> String {
        let endpoint_port = self.endpoint_port.unwrap_or_default();
        format!("{}:{}", &self.endpoint_address, endpoint_port)
    }
}

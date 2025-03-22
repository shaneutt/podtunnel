use super::{
    addresses::WireguardNetwork,
    peers::{WireguardPeer, WireguardPeerConfig},
};
use crate::helpers::ObjectReference;

use std::net::Ipv4Addr;

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, CustomResource, Debug, Default, Deserialize, JsonSchema, Serialize)]
#[kube(
    group = "podtunnel.com",
    version = "v1alpha1",
    kind = "WireguardConfig",
    namespaced,
    derive = "Default"
)]
#[serde(rename_all = "camelCase")]
#[kube(status = "WireguardConfigStatus")]
pub struct WireguardConfigSpec {
    #[serde(default)]
    pub interface: WireguardInterface,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<WireguardPeer>,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
pub struct WireguardInterface {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<WireguardAddress>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<String>>,

    #[serde(default = "crate::wireguard::default_wireguard_listen_port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key: Option<ObjectReference>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub enum WireguardAddress {
    NetworkAddress(WireguardNetwork),
    PoolAddress(ObjectReference),
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
pub struct WireguardConfigStatus {
    #[serde(default)]
    pub interface_ready: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<WireguardPeerConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_address: Option<Ipv4Addr>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tunnel_address: Option<Ipv4Addr>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tunnel_address_prefix: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<ObjectReference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

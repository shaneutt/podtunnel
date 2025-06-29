mod addresses;
mod configs;
mod peers;

pub use addresses::{
    WireguardAddressPool, WireguardAddressPoolSpec, WireguardAddressPoolStatus, WireguardNetwork,
};
pub use configs::{
    WireguardAddress, WireguardConfig, WireguardConfigSpec, WireguardConfigStatus,
    WireguardInterface,
};
pub use peers::{WireguardPeer, WireguardPeerConfig};

pub const DEFAULT_WIREGUARD_LISTEN_PORT: u16 = 51820;

fn default_wireguard_listen_port() -> Option<u16> {
    Some(DEFAULT_WIREGUARD_LISTEN_PORT)
}

use crate::controllers::errors::{Error, Result};
use api::wireguard::{
    WireguardConfig, WireguardConfigSpec, WireguardConfigStatus, WireguardInterface, WireguardPeer,
    WireguardPeerConfig,
};
use k8s_openapi::serde_json::json;

use std::sync::Arc;

use kube::{
    Api, Client, ResourceExt,
    api::{Patch, PatchParams},
    runtime::controller::Action,
};
use tracing::*;

#[derive(Clone)]
pub struct Context {
    pub client: Client,
}

pub async fn reconcile(
    wireguard_config: Arc<WireguardConfig>,
    ctx: Arc<Context>,
) -> Result<Action> {
    let name = wireguard_config.name_any();
    let namespace = wireguard_config.namespace().unwrap_or_default();
    let client = &ctx.client;

    debug!("compiling peers");
    let wireguard_configs: Api<WireguardConfig> = Api::namespaced(client.clone(), &namespace);
    let specified_peers = wireguard_config.spec.peers.iter();
    let mut compiled_peers = vec![];
    for peer in specified_peers {
        let peer_config = match peer {
            WireguardPeer::Config(config) => config,
            WireguardPeer::Pod(object_ref) => {
                &get_peer_config(&wireguard_configs, &object_ref.name).await?
            }
        };
        compiled_peers.push(peer_config.clone());
    }

    let patch = json!({
        "status": {
            "peers": compiled_peers,
        }
    });

    wireguard_configs
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .map_err(Error::KubeError)?;

    Ok(Action::await_change())
}

async fn get_peer_config(
    wireguard_configs: &Api<WireguardConfig>,
    name: &str,
) -> Result<WireguardPeerConfig> {
    match wireguard_configs.get(name).await {
        Ok(WireguardConfig {
            spec:
                WireguardConfigSpec {
                    interface: WireguardInterface { listen_port, .. },
                    ..
                },
            status:
                Some(WireguardConfigStatus {
                    interface_ready,
                    public_key: Some(public_key),
                    private_key: Some(_),
                    pod_address: Some(pod_address),
                    tunnel_address: Some(tunnel_address),
                    tunnel_address_prefix,
                    ..
                }),
            ..
        }) if interface_ready => Ok(WireguardPeerConfig {
            public_key: public_key,
            endpoint_address: pod_address,
            endpoint_port: listen_port,
            tunnel_address: Some(tunnel_address),
            tunnel_address_prefix: tunnel_address_prefix,
            allowed_ips: vec![
                format!("{}/32", &tunnel_address),
                format!("{}/32", &pod_address),
            ],
            persistent_keepalive: Some(25),
        }),
        Ok(_) => Err(Error::ControllerError(anyhow::anyhow!("peer not ready"))),
        Err(err) => Err(Error::KubeError(err)),
    }
}

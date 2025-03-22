use crate::controllers::errors::{Error, Result};
use api::{
    ObjectReference,
    wireguard::{
        WireguardAddress, WireguardAddressPool, WireguardConfig, WireguardConfigStatus,
        WireguardInterface, WireguardNetwork,
    },
};

use std::{net::Ipv4Addr, sync::Arc};

use k8s_openapi::serde_json::json;
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
    if let Some(WireguardConfigStatus { tunnel_address, .. }) = &wireguard_config.status {
        if tunnel_address.is_some() {
            debug!("address already assigned, skipping");
            return Ok(Action::await_change());
        }
    }

    let name = wireguard_config.name_any();
    let namespace = wireguard_config.namespace().unwrap_or_default();

    let assigned_address = match wireguard_config.spec.interface {
        WireguardInterface { address: None, .. } => (None, 0),
        WireguardInterface {
            address: Some(WireguardAddress::NetworkAddress(WireguardNetwork { address, prefix })),
            ..
        } => {
            info!("address configured manually");
            (Some(address), prefix)
        }
        WireguardInterface {
            address: Some(WireguardAddress::PoolAddress(ref pool)),
            ..
        } => {
            info!("assigning address from pool");
            let (address, prefix) = assign_pool_address(
                &ctx.client,
                &pool.name,
                &namespace,
                ObjectReference {
                    name: name.clone(),
                    namespace: Some(namespace.clone()),
                },
            )
            .await?;
            (Some(address), prefix)
        }
    };

    if let (Some(address), prefix) = assigned_address {
        info!("address assigned: {}/{}", &address, prefix);

        let patch = json!({
            "status": {
                "tunnel_address": address,
                "tunnel_address_prefix": prefix,
            }
        });

        let wireguard_configs: Api<WireguardConfig> =
            Api::namespaced(ctx.client.clone(), &namespace);
        wireguard_configs
            .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
            .map_err(Error::KubeError)?;
    }

    Ok(Action::await_change())
}

async fn assign_pool_address(
    client: &Client,
    pool_name: &str,
    pool_namespace: &str,
    config_ref: ObjectReference,
) -> Result<(Ipv4Addr, u8)> {
    let address_pools: Api<WireguardAddressPool> = Api::namespaced(client.clone(), &pool_namespace);

    let mut address_pool = address_pools
        .get(&pool_name)
        .await
        .map_err(Error::KubeError)?;

    let (assigned_address, prefix) = address_pool
        .assign_ipv4(config_ref)
        .await
        .map_err(Error::ControllerError)?;

    address_pools
        .patch_status(
            &address_pool.name_any(),
            &Default::default(),
            &Patch::Merge(&address_pool),
        )
        .await
        .map_err(Error::KubeError)?;

    Ok((assigned_address, prefix))
}

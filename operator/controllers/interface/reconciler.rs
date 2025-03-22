use crate::controllers::errors::{Error, Result};
use api::wireguard::{WireguardConfig, WireguardConfigStatus};
use k8s_openapi::serde_json::json;

use std::{sync::Arc, time::Duration};

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

    match &wireguard_config.status {
        Some(WireguardConfigStatus {
            interface_ready, ..
        }) if *interface_ready => {
            debug!("interface already configured, skipping");
            return Ok(Action::await_change());
        }
        Some(WireguardConfigStatus {
            pod_address: Some(_),
            tunnel_address: Some(_),
            tunnel_address_prefix: Some(_),
            private_key: Some(_),
            public_key: Some(_),
            ..
        }) => {}
        _ => {
            debug!("interface not ready yet");
            return Ok(Action::requeue(Duration::from_secs(1)));
        }
    };

    info!("interface configuration is now ready");
    let patch = json!({
        "status": {
            "interface_ready": true,
        }
    });

    let wireguard_configs: Api<WireguardConfig> = Api::namespaced(ctx.client.clone(), &namespace);
    wireguard_configs
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .map_err(Error::KubeError)?;

    Ok(Action::await_change())
}

mod reconciler;

use super::errors::{Error, Result};
use api::wireguard::WireguardConfig;
use reconciler::{Context, reconcile};

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use kube::{
    Api, Client,
    runtime::{
        Controller,
        controller::{Action, Config},
        watcher,
    },
};
use tracing::*;

pub async fn run() -> Result<(), std::io::Error> {
    let client = Client::try_default()
        .await
        .expect("failed to create kube client");

    let wireguard_configs = Api::<WireguardConfig>::all(client.clone());

    Controller::new(wireguard_configs, watcher::Config::default().any_semantic())
        .with_config(Config::default())
        .shutdown_on_signal()
        .run(
            reconcile,
            error_policy,
            Arc::new(Context { client: client }),
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => debug!("reconciled {:?}", o),
                Err(e) => debug!("reconcile failed: {}", e),
            }
        })
        .await;

    info!("ipam controller shutting down");

    Ok(())
}

fn error_policy(
    _wireguard_config: Arc<WireguardConfig>,
    _error: &Error,
    _ctx: Arc<Context>,
) -> Action {
    Action::requeue(Duration::from_secs(1))
}

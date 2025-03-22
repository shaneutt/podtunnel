use crate::controllers::{
    errors::{Error, Result},
    labels::SECRET_LABEL,
};
use api::{
    ObjectReference,
    wireguard::{WireguardConfig, WireguardConfigStatus},
};
use drivers::wireguard::key::{PrivateKey, PublicKey};

use std::{collections::BTreeMap, sync::Arc};

use k8s_openapi::{
    ByteString, api::core::v1::Secret, apimachinery::pkg::apis::meta::v1::OwnerReference,
    serde_json::json,
};
use kube::{
    Api, Client, Error as KubeError, Resource, ResourceExt,
    api::{Patch, PatchParams, PostParams},
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
    if let Some(WireguardConfigStatus { private_key, .. }) = &wireguard_config.status {
        if private_key.is_some() {
            debug!("already has a private_key, skipping");
            return Ok(Action::await_change());
        }
    }

    let name = wireguard_config.name_any();
    let namespace = wireguard_config.namespace().unwrap_or_default();

    info!("generating private key");
    let (private_key, public_key) =
        drivers::wireguard::key::generate().map_err(Error::ControllerError)?;

    let (secret_ref, _private_key, public_key) = generate_secret_ref(
        &ctx.client,
        &name,
        &namespace,
        &wireguard_config.uid().expect("no uid"),
        &private_key,
        &public_key,
    )
    .await?;

    let patch = json!({
        "status": {
            "private_key": secret_ref,
            "public_key": public_key,
        }
    });

    let wireguard_configs: Api<WireguardConfig> = Api::namespaced(ctx.client.clone(), &namespace);
    wireguard_configs
        .patch_status(&name, &PatchParams::default(), &Patch::Merge(&patch))
        .await
        .map_err(Error::KubeError)?;

    Ok(Action::await_change())
}

async fn generate_secret_ref(
    client: &Client,
    name: &str,
    namespace: &str,
    uid: &str,
    private_key: &str,
    public_key: &str,
) -> Result<(ObjectReference, PrivateKey, PublicKey)> {
    let secrets: Api<Secret> = Api::namespaced(client.clone(), &namespace);

    let map = BTreeMap::from([
        ("private_key".into(), ByteString(private_key.into())),
        ("public_key".into(), ByteString(public_key.into())),
    ]);

    let mut secret = Secret::default();
    secret.metadata.name = Some(name.to_string());
    secret.metadata.namespace = Some(namespace.to_string());
    secret.metadata.owner_references = Some(vec![OwnerReference {
        api_version: WireguardConfig::api_version(&()).into(),
        kind: WireguardConfig::kind(&()).into(),
        name: name.to_string(),
        uid: uid.to_string(),
        ..Default::default()
    }]);

    secret.metadata.labels = Some(BTreeMap::from([(
        SECRET_LABEL.to_string(),
        name.to_string(),
    )]));

    secret.data = Some(map);

    info!("generating private_key secret");
    let (private_key, public_key) = match secrets.create(&PostParams::default(), &secret).await {
        Ok(_secret) => (private_key.to_string(), public_key.to_string()),
        Err(KubeError::Api(api_err)) if api_err.code == 409 => {
            info!("private key secret for config {} already exists", &name);
            get_keys_from_secret(&secrets, name).await?
        }
        Err(e) => return Err(Error::KubeError(e)),
    };

    Ok((
        ObjectReference {
            name: name.to_string(),
            namespace: Some(namespace.to_string()),
        },
        private_key,
        public_key,
    ))
}

async fn get_keys_from_secret(
    secrets: &Api<Secret>,
    name: &str,
) -> Result<(PrivateKey, PublicKey)> {
    let secret = secrets.get(name).await.map_err(Error::KubeError)?;

    let name = secret.name_any().clone();
    let namespace = secret.namespace().unwrap_or_default().clone();

    let data = secret.data.ok_or(Error::ControllerError(anyhow::anyhow!(
        "missing data in private_key secret {}/{}",
        &namespace,
        &name,
    )))?;

    let private_key_bytestring =
        data.get("private_key")
            .ok_or(Error::ControllerError(anyhow::anyhow!(
                "missing private_key in secret {}/{}",
                &namespace,
                &name
            )))?;

    let public_key_bytestring =
        data.get("public_key")
            .ok_or(Error::ControllerError(anyhow::anyhow!(
                "missing public_key in secret {}/{}",
                &namespace,
                &name
            )))?;

    let private_key = String::from_utf8_lossy(&private_key_bytestring.0);
    let public_key = String::from_utf8_lossy(&public_key_bytestring.0);

    Ok((private_key.into_owned(), public_key.into_owned()))
}

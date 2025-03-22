use crate::{ObjectReference, helpers::Cidr};

use std::{collections::HashMap, net::Ipv4Addr};

use anyhow::Context;
use kube::{CELSchema, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct WireguardNetwork {
    pub address: Ipv4Addr,
    pub prefix: u8,
}

#[derive(CELSchema, Clone, CustomResource, Debug, Default, Deserialize, Serialize)]
#[kube(
    group = "podtunnel.com",
    version = "v1alpha1",
    kind = "WireguardAddressPool",
    namespaced,
    derive = "Default"
)]
#[serde(rename_all = "camelCase")]
#[kube(status = "WireguardAddressPoolStatus")]
pub struct WireguardAddressPoolSpec {
    #[cel_validate(
        rule = Rule::new("self.matches('^([0-9]{1,3}\\\\.){3}[0-9]{1,3}/[0-9]{1,2}$')").
        message(Message::Expression("'must be a valid IPv4 CIDR'".into())),
    )]
    #[serde(default)]
    pub network: Cidr,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
pub struct WireguardAddressPoolStatus {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub allocation: HashMap<String, std::net::Ipv4Addr>,
}

impl WireguardAddressPool {
    pub async fn assign_ipv4(
        &mut self,
        object_ref: ObjectReference,
    ) -> anyhow::Result<(Ipv4Addr, u8)> {
        let (base, prefix) = self.spec.network.split()?;

        let status = self.status.get_or_insert_default();

        let ref_key = object_ref.to_string();

        if let Some(&v) = status.allocation.get(&ref_key) {
            return Ok((v, prefix));
        }

        let base_num = u32::from(base);
        let count = 1u32 << (32 - prefix);

        let addresses = (1..count)
            .map(|i| Ipv4Addr::from(base_num + i))
            .collect::<Vec<Ipv4Addr>>();

        let address = addresses
            .iter()
            .skip_while(|&x| {
                x.is_broadcast()
                    || x.is_unspecified()
                    || status
                        .allocation
                        .values()
                        .any(|allocated_ip| allocated_ip == x)
            })
            .next()
            .context("pool exhausted")?;

        status.allocation.insert(ref_key, address.clone());

        Ok((*address, prefix))
    }
}

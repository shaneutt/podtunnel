use std::{env, io::Read, net::Ipv4Addr, str::FromStr};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::from_str as deserialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub name: String,
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "prevResult")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_result: Option<PreviousResult>,
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let mut cni_config_json = String::new();

        std::io::stdin()
            .read_to_string(&mut cni_config_json)
            .context("failed to read cni config")?;

        let cni_config =
            deserialize::<Self>(&cni_config_json).context("failed to deserialize cni config")?;

        Ok(cni_config)
    }

    pub fn extract_pod_info(&self) -> anyhow::Result<(String, String, String, Ipv4Addr)> {
        let cni_previous_result = self
            .previous_result
            .as_ref()
            .context("no cni prevresult found")?;

        let pod_netns = env::var("CNI_NETNS").context("empty CNI_NETNS")?;

        let interface_idx = cni_previous_result
            .interfaces
            .iter()
            .enumerate()
            .find_map(|(idx, interface)| match &interface.sandbox {
                Some(sandbox) if *sandbox == pod_netns => Some(idx),
                _ => None,
            })
            .context("no interface index found for cni prevresult")?;

        let cni_ip = cni_previous_result
            .ips
            .iter()
            .find(|&ip| ip.interface == Some(interface_idx))
            .context("no cni ip found for pod")?;

        let cni_ip_address = cni_ip
            .address
            .as_ref()
            .context("no address found for cni ip")?;

        let pod_ip = Ipv4Addr::from_str(
            &cni_ip_address
                .split('/')
                .next()
                .context("malformed cni ip")?,
        )?;

        let pod_netns = env::var("CNI_NETNS").context("empty CNI_NETNS")?;

        let cni_args = env::var("CNI_ARGS").context("empty CNI_ARGS")?;

        let pod_namespace =
            cni_args_extract("K8S_POD_NAMESPACE", &cni_args)?.context("empty K8S_POD_NAMESPACE")?;

        let pod_name =
            cni_args_extract("K8S_POD_NAME", &cni_args)?.context("empty K8S_POD_NAME")?;

        Ok((pod_namespace, pod_name, pod_netns, pod_ip))
    }

    pub fn print_cni_response(&self) -> anyhow::Result<()> {
        let cni_add_response = serde_json::to_string_pretty(&self.previous_result)
            .context("failed to serialize CNI prevresult")?;
        println!("{}", cni_add_response);
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreviousResult {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<Interface>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ips: Vec<Ips>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<Route>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns: Option<Dns>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Interface {
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtu: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "pciID")]
    pub pci_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ips {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Route {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dst: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gw: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtu: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advmss: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dns {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nameservers: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

fn cni_args_extract(cni_args_key: &str, cni_args: &str) -> anyhow::Result<Option<String>> {
    for pair in cni_args.split(';') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().context("malformed CNI_ARGS")?;
        let value = parts.next().context("malformed CNI_ARGS")?;

        if key == cni_args_key {
            return Ok(Some(value.to_string()));
        }
    }

    Ok(None)
}

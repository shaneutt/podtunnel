use crate::specification::{Config as CniConfig, Interface, Ips};
use drivers::wireguard::workflows::configure_wireguard_for_pod;

use anyhow::Context;

pub async fn add(cni_config: &mut CniConfig) -> anyhow::Result<()> {
    let (pod_namespace, pod_name, pod_netns, pod_ip) = cni_config
        .extract_pod_info()
        .context("failed to extract pod information from CNI environment")?;

    let (interface_name, tunnel_address, tunnel_address_prefix) =
        match configure_wireguard_for_pod(&pod_name, &pod_namespace, &pod_netns, &pod_ip).await {
            Ok(Some(result)) => result,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };

    let previous_result = cni_config
        .previous_result
        .as_mut()
        .context("prevresult is missing")?;

    previous_result.interfaces.push(Interface {
        name: interface_name,
        sandbox: Some(pod_netns),
        mac: None,
        mtu: None,
        socket_path: None,
        pci_id: None,
    });

    previous_result.ips.push(Ips {
        interface: None,
        address: Some(format!("{}/{}", &tunnel_address, tunnel_address_prefix)),
        gateway: None,
    });

    Ok(())
}

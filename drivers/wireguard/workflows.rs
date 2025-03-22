// FIXME: 🐉!here be dragons!🐉
use crate::{
    info,
    system::linux::{run, run_with_stdin},
};
use api::wireguard::{WireguardConfig, WireguardConfigStatus, WireguardPeerConfig};

use std::fs::File;
use std::net::Ipv4Addr;

use anyhow::{Context, anyhow};
use k8s_openapi::api::core::v1::Secret;
use kube::{
    Api, Client as KubeClient, Error as KubeError,
    api::{ListParams, ObjectList, Patch, PatchParams},
};
use nix::sched::{CloneFlags, setns};

const SECRET_LABEL: &str = "operator.podtunnel.com/wireguard_config";
const DEFAULT_WIREGUARD_INTERFACE_NAME: &str = "wg0";

pub async fn configure_wireguard_for_pod(
    name: &str,
    namespace: &str,
    netns: &str,
    pod_ip: &Ipv4Addr,
) -> anyhow::Result<Option<(String, Ipv4Addr, u8)>> {
    unsafe {
        std::env::set_var("KUBECONFIG", "/etc/kubernetes/admin.conf");
    };
    let kube_client = KubeClient::try_default().await?;

    info!("finding WireguardConfig for Pod {}", name);
    let wireguard_config = match get_wg_config(&kube_client, namespace, name, pod_ip).await? {
        Some(wireguard_config) => wireguard_config,
        None => return Ok(None),
    };

    info!("getting private_key for Pod{}", name);
    let private_key = get_privkey(&kube_client, name, namespace).await?;

    let (tunnel_address, tunnel_address_prefix, listen_port) = getnet(&wireguard_config);
    Ok(configure_wireguard_interface(
        netns,
        &tunnel_address,
        tunnel_address_prefix,
        &private_key,
        listen_port,
        wireguard_config.status.unwrap().peers,
    )
    .await?)
}

async fn configure_wireguard_interface(
    netns: &str,
    tunnel_address: &Ipv4Addr,
    tunnel_address_prefix: u8,
    private_key: &str,
    listen_port: u16,
    peers: Vec<WireguardPeerConfig>,
) -> anyhow::Result<Option<(String, Ipv4Addr, u8)>> {
    let original_netns_file = File::open("/proc/self/ns/net")?;
    let container_netns_file = File::open(&netns)?;

    info!("pod netns: {:?}", &container_netns_file);
    setns(&container_netns_file, CloneFlags::CLONE_NEWNET)?;

    info!("adding wireguard interface");
    run(
        "ip",
        vec![
            "link",
            "add",
            DEFAULT_WIREGUARD_INTERFACE_NAME,
            "type",
            "wireguard",
        ],
    )?;

    info!("adding address {} to wireguard interface", &tunnel_address);
    run(
        "ip",
        vec![
            "addr",
            "add",
            &format!("{}/{}", tunnel_address, tunnel_address_prefix),
            "dev",
            DEFAULT_WIREGUARD_INTERFACE_NAME,
        ],
    )?;

    info!("configuring private key for wireguard interface");
    run_with_stdin(
        "wg",
        vec![
            "set",
            DEFAULT_WIREGUARD_INTERFACE_NAME,
            "private-key",
            "/dev/stdin",
        ],
        &private_key,
    )?;

    info!("setting the wireguard listen-port {}", listen_port);
    run(
        "wg",
        vec![
            "set",
            DEFAULT_WIREGUARD_INTERFACE_NAME,
            "listen-port",
            &listen_port.to_string(),
        ],
    )?;

    info!("bringing the wireguard interface up");
    run(
        "ip",
        vec!["link", "set", DEFAULT_WIREGUARD_INTERFACE_NAME, "up"],
    )?;

    info!("setting the fwmark");
    let fwmark = "921481285";
    run(
        "wg",
        vec!["set", DEFAULT_WIREGUARD_INTERFACE_NAME, "fwmark", &fwmark],
    )?;

    info!("adding a custom routing table");
    let routing_table = "129518285";
    run(
        "ip",
        vec![
            "route",
            "add",
            "default",
            "dev",
            DEFAULT_WIREGUARD_INTERFACE_NAME,
            "table",
            &routing_table,
        ],
    )?;

    info!("configuring peers");
    for peer in peers {
        info!("configuring peer {}", &peer.public_key);
        run(
            "wg",
            vec![
                "set",
                DEFAULT_WIREGUARD_INTERFACE_NAME,
                "peer",
                &peer.public_key,
                "allowed-ips",
                &peer.allowed_ips.join(","),
                "endpoint",
                &peer.endpoint(),
            ],
        )?;

        info!("routing wireguard traffic to the main routing table");
        run(
            "ip",
            vec![
                "rule",
                "add",
                "from",
                "all",
                "to",
                &peer.endpoint_address.to_string(),
                "fwmark",
                &fwmark,
                "lookup",
                "main",
                "priority",
                "1",
            ],
        )?;

        info!("routing regular traffic over the wireguard tunnel");
        run(
            "ip",
            vec![
                "rule",
                "add",
                "from",
                "all",
                "to",
                &peer.endpoint_address.to_string(),
                "fwmark",
                "0",
                "lookup",
                &routing_table,
                "priority",
                "2",
            ],
        )?;
    }

    info!("ensure most specific routing rules match");
    run(
        "ip",
        vec![
            "rule",
            "add",
            "table",
            "main",
            "suppress_prefixlength",
            "0",
            "priority",
            "3",
        ],
    )?;

    info!("returning back to original netns");
    setns(original_netns_file, CloneFlags::CLONE_NEWNET)?;

    Ok(Some((
        DEFAULT_WIREGUARD_INTERFACE_NAME.to_string(),
        *tunnel_address,
        tunnel_address_prefix,
    )))
}

async fn get_wg_config(
    kube_client: &KubeClient,
    namespace: &str,
    name: &str,
    pod_ip: &Ipv4Addr,
) -> anyhow::Result<Option<WireguardConfig>> {
    let wireguard_configs: Api<WireguardConfig> = Api::namespaced(kube_client.clone(), &namespace);
    let wireguard_config = 'wait_for_readiness: loop {
        let mut wireguard_config = match wireguard_configs.get(&name).await {
            Ok(wireguard_config) => wireguard_config,
            Err(KubeError::Api(api_err)) if api_err.code == 404 => return Ok(None),
            Err(err) => return Err(err.into()),
        };

        let status = wireguard_config.status.get_or_insert_default();
        status.pod_address = Some(*pod_ip);

        match wireguard_configs
            .patch_status(
                name,
                &PatchParams::default(),
                &Patch::Merge(&wireguard_config),
            )
            .await
        {
            Ok(_) => {}
            Err(_) => continue 'wait_for_readiness,
        }

        if let WireguardConfig {
            status:
                Some(WireguardConfigStatus {
                    interface_ready,
                    ref peers,
                    ..
                }),
            ..
        } = wireguard_config
        {
            if interface_ready && peers.len() > 0 {
                break wireguard_config;
            }
        }
    };

    Ok(Some(wireguard_config))
}

async fn get_privkey(
    kube_client: &KubeClient,
    name: &str,
    namespace: &str,
) -> anyhow::Result<String> {
    let secrets: Api<Secret> = Api::namespaced(kube_client.clone(), namespace);
    let private_key_secret_label_selector = format!("{}={}", SECRET_LABEL, name);
    let private_key_secret = match secrets
        .list(&ListParams::default().labels(&private_key_secret_label_selector))
        .await?
    {
        object_list if object_list.items.len() == 1 => object_list.items[0].clone(),
        ObjectList { items, .. } => return Err(anyhow!("expected 1 secret found {}", items.len())),
    };

    let data = private_key_secret.data.context("missing secret data")?;
    Ok(String::from_utf8_lossy(&data.get("private_key").context("missing private_key")?.0).into())
}

fn getnet(wireguard_config: &WireguardConfig) -> (Ipv4Addr, u8, u16) {
    let tunnel_address = wireguard_config
        .status
        .as_ref()
        .unwrap()
        .tunnel_address
        .unwrap();
    let prefix = wireguard_config
        .status
        .as_ref()
        .unwrap()
        .tunnel_address_prefix
        .unwrap();
    let listen_port = wireguard_config
        .spec
        .interface
        .listen_port
        .unwrap_or_default();
    (tunnel_address, prefix, listen_port)
}

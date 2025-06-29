use api::wireguard::{WireguardAddressPool, WireguardConfig};

use std::{env, fs};

use clap::Parser;
use k8s_openapi::{
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
    serde_json::{self, json},
};
use kube::{CustomResourceExt, ResourceExt};

const CNI_NAME: &str = "podtunnel-cni";
const CNI_TYPE: &str = "podtunnel-cni";
const CNI_VERSION: &str = "0.3.1";
const CRD_KUSTOMIZE_DIR: &str = "config/crds";

#[derive(Debug, Parser)]
enum Command {
    GenerateCniConfig,
    GenerateCrds,
}

#[derive(Debug, Parser)]
struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = Options::parse();
    use Command::*;
    match opts.command {
        GenerateCniConfig => {
            let cni_config_json = serde_json::to_string_pretty(
                &(json!({
                    "name":  env::var("CNI_NAME").unwrap_or(CNI_NAME.to_string()),
                    "cniVersion":  env::var("CNI_TYPE").unwrap_or(CNI_TYPE.to_string()),
                    "type": env::var("CNI_VERSION").unwrap_or(CNI_VERSION.to_string()),
                })),
            )?;
            println!("{}", cni_config_json);
            Ok(())
        }
        GenerateCrds => {
            let crds = vec![WireguardAddressPool::crd(), WireguardConfig::crd()];
            let crd_file_names = create_crd_files(crds)?;
            create_kustomization_file(crd_file_names)?;
            Ok(())
        }
    }
}

fn create_crd_files(crds: Vec<CustomResourceDefinition>) -> anyhow::Result<Vec<String>> {
    let mut crd_file_names: Vec<String> = Vec::new();
    for crd in crds {
        let crd_file_name = format!("{}.yaml", crd.name_any().to_lowercase().replace(".", "_"),);

        let crd_file_path = format!("{}/{}", CRD_KUSTOMIZE_DIR, &crd_file_name);
        let current_file_contents = fs::read_to_string(&crd_file_path).unwrap_or_default();
        let new_file_contents = serde_yaml::to_string(&crd)?;

        if current_file_contents == new_file_contents {
            println!("{} already up-to-date", &crd_file_path);
        } else {
            std::fs::write(&crd_file_path, &new_file_contents)?;
            println!("wrote: {}", &crd_file_path);
        }

        crd_file_names.push(crd_file_name);
    }
    Ok(crd_file_names)
}

fn create_kustomization_file(crd_file_names: Vec<String>) -> anyhow::Result<()> {
    let kustomize_file_path = format!("{}/kustomization.yaml", CRD_KUSTOMIZE_DIR);
    let current_file_contents = fs::read_to_string(&kustomize_file_path).unwrap_or_default();
    let mut new_file_contents = r#"---
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:"#
        .to_string();
    for file_name in crd_file_names {
        new_file_contents.push_str("\n- ");
        new_file_contents.push_str(&file_name);
    }
    new_file_contents.push_str("\n");

    if current_file_contents == new_file_contents {
        println!("{} already up-to-date", &kustomize_file_path);
    } else {
        std::fs::write(&kustomize_file_path, new_file_contents)?;
        println!("wrote: {}", kustomize_file_path);
    }

    Ok(())
}

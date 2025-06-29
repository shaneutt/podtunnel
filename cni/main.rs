mod operations;
mod specification;

use drivers::info;
use operations as cni;
use specification::Config as CniConfig;

use std::env;

use anyhow::{Context, anyhow};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    info!("running podtunnel cni");

    let cni_command = env::var("CNI_COMMAND").context("no CNI_COMMAND provided")?;
    let mut cni_config = CniConfig::new()?;
    match cni_command.as_str() {
        "ADD" => cni::add(&mut cni_config).await?,
        "DEL" => cni::del(&mut cni_config)?,
        "CHECK" => cni::check(&mut cni_config)?,
        _ => return Err(anyhow!("unsupported cni command {}", cni_command)),
    }

    cni_config.print_cni_response()?;

    Ok(())
}

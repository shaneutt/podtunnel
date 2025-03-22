use crate::system::linux::{run, run_with_stdin};

use anyhow::Context;

pub type PrivateKey = String;
pub type PublicKey = String;

pub fn generate() -> anyhow::Result<(PrivateKey, PublicKey)> {
    let private_key = run("wg", vec!["genkey"]).context("private key generation failed")?;
    let public_key = run_with_stdin("wg", vec!["pubkey"], &private_key)
        .context("public key generation failed")?;
    Ok((private_key, public_key))
}

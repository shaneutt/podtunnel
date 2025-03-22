use crate::specification::Config as CniConfig;

pub fn check(cni_config: &mut CniConfig) -> anyhow::Result<()> {
    println!("CNI CHECK UNIMPLEMENTED, CONFIG: {:?}", cni_config);
    Ok(())
}

use crate::specification::Config as CniConfig;

pub fn del(cni_config: &mut CniConfig) -> anyhow::Result<()> {
    println!("CNI DELETE UNIMPLEMENTED, CONFIG: {:?}", cni_config);
    Ok(())
}

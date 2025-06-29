use core::fmt;
use std::str::FromStr;
use std::{
    fmt::{Display, Formatter},
    net::Ipv4Addr,
};

use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const DEFAULT_NAMESPACE: &str = "default";
const DEFAULT_NETWORK: &str = "10.0.100.0/24";

#[derive(Clone, Debug, Eq, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Cidr(String);
impl Cidr {
    pub fn split(&self) -> anyhow::Result<(Ipv4Addr, u8)> {
        let parts: Vec<&str> = self.0.split('/').collect();
        match parts.len() {
            2 => {
                let base = Ipv4Addr::from_str(parts[0])?;
                let prefix = parts[1].parse::<u8>()?;
                Ok((base, prefix))
            }
            _ => Err(anyhow!("invalid cidr {}", self.0)),
        }
    }
}

impl Default for Cidr {
    fn default() -> Self {
        Cidr(DEFAULT_NETWORK.to_string())
    }
}

impl Display for Cidr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Deserialize, Eq, Serialize, Clone, Debug, Default, Hash, JsonSchema, PartialEq)]
pub struct ObjectReference {
    pub name: String,

    #[serde(default = "default_namespace")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl Display for ObjectReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let namespace = self.namespace.as_deref().unwrap_or_default();
        write!(f, "{}/{}", namespace, &self.name)
    }
}

fn default_namespace() -> Option<String> {
    Some(DEFAULT_NAMESPACE.to_string())
}

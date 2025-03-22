use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("controller error: {0}")]
    ControllerError(#[source] anyhow::Error),
    #[error("kube error: {0}")]
    KubeError(#[source] kube::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid geometry: {0}")]
    InvalidGeometry(String),

    #[error("topology error: {0}")]
    TopologyError(String),

    #[error("parse error: {0}")]
    ParseError(String),
}

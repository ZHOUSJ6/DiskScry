pub mod ata;
pub mod nvme;

use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("{structure} requires at least {expected} bytes, got {actual}")]
    Truncated {
        structure: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("{structure} checksum is invalid")]
    InvalidChecksum { structure: &'static str },
}

pub(crate) fn require_len(
    bytes: &[u8],
    expected: usize,
    structure: &'static str,
) -> Result<(), ParseError> {
    if bytes.len() < expected {
        return Err(ParseError::Truncated {
            structure,
            expected,
            actual: bytes.len(),
        });
    }
    Ok(())
}

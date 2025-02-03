
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ConversionError {
    InvalidProgramId,
    InvalidData,
    UnsupportedInstructionFormat,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidProgramId => write!(f, "Invalid program id"),
            Self::InvalidData => write!(f, "Invalid data"),
            Self::UnsupportedInstructionFormat => write!(f, "Cannot convert from UiInstruction::Parsed to CompiledInstruction"),
        }
    }
}

impl Error for ConversionError {}

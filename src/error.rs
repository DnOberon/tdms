use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
/// A set of library specific errors.
pub enum TdmsError {
    #[error("{0:?}")]
    ReadError(#[from] io::Error),

    #[error("error while reading .tdms file {0}")]
    General(String),

    #[error("invalid segment - malformed or missing lead-in tag")]
    InvalidSegment(),

    #[error("unsupported version, only version 4173 supported")]
    UnsupportedVersion(),

    #[error("unable to convert to String")]
    StringConversionError(),

    #[error("unknown data type")]
    UnknownDataType(),

    #[error("not implemented")]
    NotImplemented,
}

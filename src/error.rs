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

    #[error("invalid DAQmx data index")]
    InvalidDAQmxDataIndex(),

    #[error("unable to convert {0} to String")]
    StringConversionError(String),

    #[error("unknown data type")]
    UnknownDataType(),

    #[error("not implemented")]
    NotImplemented,
}

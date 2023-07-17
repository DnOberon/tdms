use std::io;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Error, Debug)]
/// A set of library specific errors.
pub enum TdmsError {
    #[error("{0:?}")]
    ReadError(#[from] io::Error),

    #[error("{0:?}")]
    IntConversionError(#[from] TryFromIntError),

    #[error("error while reading .tdms file {0}")]
    General(String),

    #[error("invalid segment - malformed or missing lead-in tag")]
    InvalidSegment(),


    #[error("requested group does not exist in segment")]
    GroupDoesNotExist(),

    #[error("requested channel does not exist in segment")]
    ChannelDoesNotExist(),

    #[error("end of segments in file reached")]
    EndOfSegments(),

    #[error("invalid DAQmx data index")]
    InvalidDAQmxDataIndex(),

    #[error("unable to convert {0} to String")]
    StringConversionError(String),

    #[error("unknown data type")]
    UnknownDataType(),

    #[error("{0} not implemented")]
    NotImplemented(String),
}

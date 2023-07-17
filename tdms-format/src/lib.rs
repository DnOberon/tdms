pub mod data_type;
pub mod error;
pub mod segment;

pub use crate::TdmsError::{
    General, InvalidDAQmxDataIndex, InvalidSegment, StringConversionError, UnknownDataType,
};
pub use error::TdmsError;
pub use segment::Endianness::{Big, Little};
pub use segment::{Endianness, Segment};

//! A Rust library for reading LabView TDMS files.
//!
//! More information about the TDMS file format can be found here: <https://www.ni.com/en-us/support/documentation/supplemental/07/tdms-file-format-internal-structure.html>
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub mod error;
use crate::TdmsError::{
    General, InvalidDAQmxDataIndex, InvalidSegment, StringConversionError, UnknownDataType,
};
pub use error::TdmsError;
use segment::Endianness::{Big, Little};
use segment::{Endianness, Segment};

pub mod segment;
#[cfg(test)]
mod tests;

/// Represents the potential TDMS data types .
#[derive(Debug, Copy, Clone)]
pub enum TdmsDataType {
    Void,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    SingleFloat,
    DoubleFloat,
    ExtendedFloat,
    SingleFloatWithUnit = 0x19,
    DoubleFloatWithUnit = 0x1a,
    ExtendedFloatWithUnit = 0x1b,
    String = 0x20,
    Boolean = 0x21,
    TimeStamp = 0x44,
    FixedPoint = 0x4F,
    ComplexSingleFloat = 0x08000c,
    ComplexDoubleFloat = 0x10000d,
    DAQmxRawData = 0xFFFFFFFF,
}

impl TryFrom<i32> for TdmsDataType {
    type Error = TdmsError;

    fn try_from(v: i32) -> Result<Self, TdmsError> {
        match v {
            x if x == TdmsDataType::Void as i32 => Ok(TdmsDataType::Void),
            x if x == TdmsDataType::I8 as i32 => Ok(TdmsDataType::I8),
            x if x == TdmsDataType::I16 as i32 => Ok(TdmsDataType::I16),
            x if x == TdmsDataType::I32 as i32 => Ok(TdmsDataType::I32),
            x if x == TdmsDataType::I64 as i32 => Ok(TdmsDataType::I64),
            x if x == TdmsDataType::U8 as i32 => Ok(TdmsDataType::U8),
            x if x == TdmsDataType::U16 as i32 => Ok(TdmsDataType::U16),
            x if x == TdmsDataType::U32 as i32 => Ok(TdmsDataType::U32),
            x if x == TdmsDataType::U64 as i32 => Ok(TdmsDataType::U64),
            x if x == TdmsDataType::SingleFloat as i32 => Ok(TdmsDataType::SingleFloat),
            x if x == TdmsDataType::DoubleFloat as i32 => Ok(TdmsDataType::DoubleFloat),
            x if x == TdmsDataType::ExtendedFloat as i32 => Ok(TdmsDataType::ExtendedFloat),
            x if x == TdmsDataType::SingleFloatWithUnit as i32 => {
                Ok(TdmsDataType::SingleFloatWithUnit)
            }
            x if x == TdmsDataType::DoubleFloatWithUnit as i32 => {
                Ok(TdmsDataType::DoubleFloatWithUnit)
            }
            x if x == TdmsDataType::ExtendedFloatWithUnit as i32 => {
                Ok(TdmsDataType::ExtendedFloatWithUnit)
            }
            x if x == TdmsDataType::String as i32 => Ok(TdmsDataType::String),
            x if x == TdmsDataType::Boolean as i32 => Ok(TdmsDataType::Boolean),
            x if x == TdmsDataType::TimeStamp as i32 => Ok(TdmsDataType::TimeStamp),
            x if x == TdmsDataType::FixedPoint as i32 => Ok(TdmsDataType::FixedPoint),
            x if x == TdmsDataType::ComplexSingleFloat as i32 => {
                Ok(TdmsDataType::ComplexSingleFloat)
            }
            x if x == TdmsDataType::ComplexDoubleFloat as i32 => {
                Ok(TdmsDataType::ComplexDoubleFloat)
            }
            x if x == TdmsDataType::DAQmxRawData as i32 => Ok(TdmsDataType::DAQmxRawData),
            _ => Err(UnknownDataType()),
        }
    }
}

#[derive(Debug)]
/// `TDDMSFile` represents all `segments` of a TDMS file in the order in which they were read.
pub struct TDMSFile {
    pub segments: Vec<Segment>,
}

impl TDMSFile {
    /// `from_path` expects a path and whether or not to read only the metadata of each segment vs
    /// the entire file into working memory.
    pub fn from_path(path: &str, metadata_only: bool) -> Result<Self, TdmsError> {
        let metadata = fs::metadata(Path::new(path))?;
        let mut file = File::open(Path::new(path))?;
        let mut segments: Vec<Segment> = vec![];

        loop {
            let segment = Segment::new(&mut file, metadata_only)?;

            if segment.end_pos == metadata.len() {
                segments.push(segment);
                break;
            }

            file.seek(SeekFrom::Start(segment.end_pos))?;
            segments.push(segment);
        }

        return Ok(TDMSFile { segments });
    }
}

#[derive(Debug)]
/// `TDMSValue` represents a single value read from a TDMS file. This contains information on the
/// data type and the endianness of the value if numeric. This is typically used only by segment
/// and in the metadata properties, as using these for raw values is not good for performance.
pub struct TDMSValue {
    pub data_type: TdmsDataType,
    pub endianness: Endianness,
    pub value: Option<Vec<u8>>,
}

impl TDMSValue {
    /// from_file accepts an open file and a data type and attempts to read the file, generating a
    /// value struct containing the actual value
    pub fn from_file(
        endianness: Endianness,
        data_type: TdmsDataType,
        file: &mut File,
    ) -> Result<Self, TdmsError> {
        return match data_type {
            TdmsDataType::Void => Ok(TDMSValue {
                data_type,
                endianness,
                value: None,
            }),
            TdmsDataType::I8 => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I16 => {
                let mut buf: [u8; 2] = [0; 2];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I32 => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I64 => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U8 => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U16 => {
                let mut buf: [u8; 2] = [0; 2];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U32 => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U64 => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::SingleFloat => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DoubleFloat => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ExtendedFloat => {
                let mut buf: [u8; 10] = [0; 10];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::SingleFloatWithUnit => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DoubleFloatWithUnit => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ExtendedFloatWithUnit => {
                let mut buf: [u8; 10] = [0; 10];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::String => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                let length: u32 = match endianness {
                    Little => u32::from_le_bytes(buf),
                    Big => u32::from_be_bytes(buf),
                };

                // must be a vec due to variable length
                let length = match usize::try_from(length) {
                    Ok(l) => l,
                    Err(_) => {
                        return Err(General(String::from(
                            "error converting strength length to system size",
                        )))
                    }
                };

                let mut value = vec![0; length];
                file.read(&mut value)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(value),
                })
            }
            TdmsDataType::Boolean => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::TimeStamp => {
                let mut buf: [u8; 16] = [0; 16];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            // there is little information on how to handle FixedPoint types, for
            // now we'll store them as a 64 bit integer and hope that will be enough
            TdmsDataType::FixedPoint => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ComplexSingleFloat => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ComplexDoubleFloat => {
                let mut buf: [u8; 16] = [0; 16];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DAQmxRawData => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
        };
    }
}

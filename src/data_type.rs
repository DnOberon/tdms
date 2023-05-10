use crate::{Big, Endianness, General, Little, TdmsError, UnknownDataType};
#[cfg(feature = "chrono")]
use chrono::{prelude::*, Duration};
use std::io::{Read, Seek};
#[cfg(feature = "time")]
use time::{macros::datetime, Duration, PrimitiveDateTime};

/// Represents the potential TDMS data types. Contained value is size in bytes if applicable
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TdmsDataType {
    Void,
    I8(usize),
    I16(usize),
    I32(usize),
    I64(usize),
    U8(usize),
    U16(usize),
    U32(usize),
    U64(usize),
    SingleFloat(usize),
    DoubleFloat(usize),
    ExtendedFloat(usize),
    SingleFloatWithUnit(usize),
    DoubleFloatWithUnit(usize),
    ExtendedFloatWithUnit(usize),
    String,
    Boolean(usize),
    TimeStamp(usize),
    FixedPoint(usize),
    ComplexSingleFloat(usize),
    ComplexDoubleFloat(usize),
    DAQmxRawData,
}

impl TryFrom<i32> for TdmsDataType {
    type Error = TdmsError;

    fn try_from(v: i32) -> Result<Self, TdmsError> {
        match v {
            x if x == 0 => Ok(TdmsDataType::Void),
            x if x == 1 => Ok(TdmsDataType::I8(1)),
            x if x == 2 => Ok(TdmsDataType::I16(2)),
            x if x == 3 => Ok(TdmsDataType::I32(4)),
            x if x == 4 => Ok(TdmsDataType::I64(8)),
            x if x == 5 => Ok(TdmsDataType::U8(1)),
            x if x == 6 => Ok(TdmsDataType::U16(2)),
            x if x == 7 => Ok(TdmsDataType::U32(4)),
            x if x == 8 => Ok(TdmsDataType::U64(8)),
            x if x == 9 => Ok(TdmsDataType::SingleFloat(4)),
            x if x == 10 => Ok(TdmsDataType::DoubleFloat(8)),
            x if x == 11 => Ok(TdmsDataType::ExtendedFloat(10)),
            x if x == 0x19 => Ok(TdmsDataType::SingleFloatWithUnit(4)),
            x if x == 0x1a => Ok(TdmsDataType::DoubleFloatWithUnit(8)),
            x if x == 0x1b => Ok(TdmsDataType::ExtendedFloatWithUnit(10)),
            x if x == 0x20 => Ok(TdmsDataType::String),
            x if x == 0x21 => Ok(TdmsDataType::Boolean(1)),
            x if x == 0x44 => Ok(TdmsDataType::TimeStamp(16)),
            x if x == 0x4f => Ok(TdmsDataType::FixedPoint(10)),
            x if x == 0x08000c => Ok(TdmsDataType::ComplexSingleFloat(4)),
            x if x == 0x10000d => Ok(TdmsDataType::ComplexDoubleFloat(8)),
            x if x == -1 => Ok(TdmsDataType::DAQmxRawData), // 0xFFFFFFFF equivalent
            _ => Err(UnknownDataType()),
        }
    }
}

impl TdmsDataType {
    pub fn get_size(data_type: TdmsDataType) -> usize {
        return match data_type {
            TdmsDataType::Void => 0,
            TdmsDataType::I8(v) => v,
            TdmsDataType::I16(v) => v,
            TdmsDataType::I32(v) => v,
            TdmsDataType::I64(v) => v,
            TdmsDataType::U8(v) => v,
            TdmsDataType::U16(v) => v,
            TdmsDataType::U32(v) => v,
            TdmsDataType::U64(v) => v,
            TdmsDataType::SingleFloat(v) => v,
            TdmsDataType::DoubleFloat(v) => v,
            TdmsDataType::ExtendedFloat(v) => v,
            TdmsDataType::SingleFloatWithUnit(v) => v,
            TdmsDataType::DoubleFloatWithUnit(v) => v,
            TdmsDataType::ExtendedFloatWithUnit(v) => v,
            TdmsDataType::String => 0,
            TdmsDataType::Boolean(v) => v,
            TdmsDataType::TimeStamp(v) => v,
            TdmsDataType::FixedPoint(v) => v,
            TdmsDataType::ComplexSingleFloat(v) => v,
            TdmsDataType::ComplexDoubleFloat(v) => v,
            TdmsDataType::DAQmxRawData => 0,
        };
    }
}

#[derive(Debug, Clone)]
/// `TDMSValue` represents a single value read from a TDMS file. This contains information on the
/// data type and the endianness of the value if numeric. This is typically used only by segment
/// and in the metadata properties, as using these for raw values is not good for performance.
pub struct TDMSValue {
    pub data_type: TdmsDataType,
    pub endianness: Endianness,
    pub value: Option<Vec<u8>>,
}

impl TDMSValue {
    /// from_reader accepts an open reader and a data type and attempts to read, generating a
    /// value struct containing the actual value
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        data_type: TdmsDataType,
        r: &mut R,
    ) -> Result<Self, TdmsError> {
        return match data_type {
            TdmsDataType::Void => Ok(TDMSValue {
                data_type,
                endianness,
                value: None,
            }),
            TdmsDataType::I8(_) => {
                let mut buf: [u8; 1] = [0; 1];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I16(_) => {
                let mut buf: [u8; 2] = [0; 2];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I32(_) => {
                let mut buf: [u8; 4] = [0; 4];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::I64(_) => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U8(_) => {
                let mut buf: [u8; 1] = [0; 1];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U16(_) => {
                let mut buf: [u8; 2] = [0; 2];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U32(_) => {
                let mut buf: [u8; 4] = [0; 4];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::U64(_) => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::SingleFloat(_) => {
                let mut buf: [u8; 4] = [0; 4];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DoubleFloat(_) => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ExtendedFloat(_) => {
                let mut buf: [u8; 10] = [0; 10];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::SingleFloatWithUnit(_) => {
                let mut buf: [u8; 4] = [0; 4];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DoubleFloatWithUnit(_) => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ExtendedFloatWithUnit(_) => {
                let mut buf: [u8; 10] = [0; 10];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::String => {
                let mut buf: [u8; 4] = [0; 4];
                r.read_exact(&mut buf)?;

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
                r.read_exact(&mut value)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(value),
                })
            }
            TdmsDataType::Boolean(_) => {
                let mut buf: [u8; 1] = [0; 1];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::TimeStamp(_) => {
                let mut buf: [u8; 16] = [0; 16];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            // there is little information on how to handle FixedPoint types, for
            // now we'll store them as a 64 bit integer and hope that will be enough
            TdmsDataType::FixedPoint(_) => {
                let mut buf: [u8; 10] = [0; 10];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ComplexSingleFloat(_) => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::ComplexDoubleFloat(_) => {
                let mut buf: [u8; 16] = [0; 16];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
            TdmsDataType::DAQmxRawData => {
                let mut buf: [u8; 8] = [0; 8];
                r.read_exact(&mut buf)?;

                Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                })
            }
        };
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TdmsTimestamp {
    pub seconds_since_ni_epoch: i64,
    pub fractions_of_a_second: u64,
}

#[cfg(feature = "time")]
impl TdmsTimestamp {
    const NI_EPOCH: PrimitiveDateTime = datetime!(1904-01-01 00:00);

    pub fn to_duration(&self) -> Duration {
        Duration::seconds(self.seconds_since_ni_epoch)
            + Duration::seconds_f64(self.fractions_of_a_second as f64 / u64::MAX as f64)
    }

    pub fn to_primitive_date_time(&self) -> PrimitiveDateTime {
        TdmsTimestamp::NI_EPOCH + self.to_duration()
    }
}

#[cfg(feature = "chrono")]
impl TdmsTimestamp {
    const NI_EPOCH: NaiveDateTime = NaiveDate::from_ymd_opt(1904, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    pub fn to_duration(&self) -> Duration {
        Duration::seconds(self.seconds_since_ni_epoch)
            + (Duration::from_std(std::time::Duration::from_secs_f64(
                self.fractions_of_a_second as f64 / u64::MAX as f64,
            ))
            .unwrap())
    }

    pub fn to_naive_date_time(&self) -> NaiveDateTime {
        TdmsTimestamp::NI_EPOCH + self.to_duration()
    }
}

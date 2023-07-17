use crate::{
    Endianness,
    TdmsError::{self, General},
    UnknownDataType,
};

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
    pub fn from_reader(
        endianness: Endianness,
        data_type: TdmsDataType,
        r: &[u8],
    ) -> Result<(Self, &[u8]), TdmsError> {
        return match data_type {
            TdmsDataType::Void => Ok((
                TDMSValue {
                    data_type,
                    endianness,
                    value: None,
                },
                r,
            )),
            TdmsDataType::I8(_) => {
                let (buf, rest) = r.split_at(1);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::I16(_) => {
                let (buf, rest) = r.split_at(2);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::I32(_) => {
                let (buf, rest) = r.split_at(4);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::I64(_) => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::U8(_) => {
                let (buf, rest) = r.split_at(1);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::U16(_) => {
                let (buf, rest) = r.split_at(2);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::U32(_) => {
                let (buf, rest) = r.split_at(4);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::U64(_) => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::SingleFloat(_) => {
                let (buf, rest) = r.split_at(4);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::DoubleFloat(_) => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::ExtendedFloat(_) => {
                let (buf, rest) = r.split_at(10);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::SingleFloatWithUnit(_) => {
                let (buf, rest) = r.split_at(4);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::DoubleFloatWithUnit(_) => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::ExtendedFloatWithUnit(_) => {
                let (buf, rest) = r.split_at(10);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::String => {
                let (buf, rest) = r.split_at(4);

                if let Ok(buf) = buf.try_into() {
                    let length: u32 = match endianness {
                        Endianness::Little => u32::from_le_bytes(buf),
                        Endianness::Big => u32::from_be_bytes(buf),
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

                    let (value, rest) = rest.split_at(length);

                    Ok((
                        TDMSValue {
                            data_type,
                            endianness,
                            value: Some(value.to_vec()),
                        },
                        rest,
                    ))
                } else {
                    Err(TdmsError::General(String::from(
                        "buffer insufficiently long to contain string value",
                    )))
                }
            }
            TdmsDataType::Boolean(_) => {
                let (buf, rest) = r.split_at(1);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::TimeStamp(_) => {
                let (buf, rest) = r.split_at(16);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            // there is little information on how to handle FixedPoint types, for
            // now we'll store them as a 64 bit integer and hope that will be enough
            TdmsDataType::FixedPoint(_) => {
                let (buf, rest) = r.split_at(10);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::ComplexSingleFloat(_) => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::ComplexDoubleFloat(_) => {
                let (buf, rest) = r.split_at(16);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
            TdmsDataType::DAQmxRawData => {
                let (buf, rest) = r.split_at(8);

                Ok((
                    TDMSValue {
                        data_type,
                        endianness,
                        value: Some(buf.to_vec()),
                    },
                    rest,
                ))
            }
        };
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TdmsTimestamp(pub i64, pub u64);

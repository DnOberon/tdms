//! A Rust library for reading LabView TDMS files.
//!
//! More information about the TDMS file format can be found here: <https://www.ni.com/en-us/support/documentation/supplemental/07/tdms-file-format-internal-structure.html>
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

mod error;
use crate::Endianness::{Big, Little};
use crate::TdmsError::{
    General, InvalidDAQmxDataIndex, InvalidSegment, StringConversionError, UnknownDataType,
};
pub use error::TdmsError;

#[cfg(test)]
mod tests;

/// These are bitmasks for the Table of Contents byte.
const K_TOC_META_DATA: u32 = 1 << 1;
const K_TOC_NEW_OBJ_LIST: u32 = 1 << 2;
const K_TOC_RAW_DATA: u32 = 1 << 3;
const K_TOC_INTERLEAVED_DATA: u32 = 1 << 5;
const K_TOC_BIG_ENDIAN: u32 = 1 << 6;
const K_TOC_DAQMX_RAW_DATA: u32 = 1 << 7;

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

/// Ease of use enum for determining how to read numerical values.
#[derive(Clone, Copy, Debug)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug)]
/// `TDDMSFile` represents all Segments of a TDMS file in the order in which they were read.
pub struct TDMSFile {
    segments: Vec<Segment>,
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
/// `Segment` represents an entire TDMS File Segment and potentially its raw data.
pub struct Segment {
    lead_in: LeadIn,
    // TODO: remove Option when actually ready to parse
    metadata: Option<Metadata>,
    // TODO: remove Option when actually ready to parse
    raw_data: Option<Vec<u8>>,
    start_pos: u64,
    end_pos: u64,
}

impl Segment {
    /// New expects a file who's cursor position is at the start of a new TDMS segment.
    /// You will see an InvalidSegment error return if the file position isn't correct as the first
    /// byte read will not be the correct tag for a segment.
    pub fn new(file: &mut File, metadata_only: bool) -> Result<Self, TdmsError> {
        let start_pos = file.stream_position()?;
        let mut lead_in = [0; 28];

        file.read(&mut lead_in[..])?;

        let lead_in = LeadIn::from_bytes(&lead_in)?;

        // calculate the end position by taking the start and adding the offset plus lead in bytes
        let end_pos = lead_in.next_segment_offset + 28 + start_pos;

        let endianness = if lead_in.table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            Big
        } else {
            Little
        };

        let metadata = Metadata::from_file(endianness, file)?;
        let data: Vec<u8> = vec![];

        return Ok(Segment {
            lead_in,
            metadata: Some(metadata),
            raw_data: if metadata_only { None } else { Some(data) },
            start_pos,
            /// lead in plus offset
            end_pos,
        });
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn endianess(&self) -> Endianness {
        return if self.lead_in.table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            Big
        } else {
            Little
        };
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn interleaved_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_INTERLEAVED_DATA == 1;
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn daqmx_raw_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_DAQMX_RAW_DATA == 1;
    }
}

#[derive(Debug)]
/// `LeadIn` represents the 28 bytes representing the lead in to a TDMS Segment.
pub struct LeadIn {
    tag: [u8; 4],
    table_of_contents: u32,
    version_number: u32,
    next_segment_offset: u64,
    raw_data_offset: u64,
}

impl LeadIn {
    pub fn from_bytes(lead_in: &[u8; 28]) -> Result<Self, TdmsError> {
        let mut tag: [u8; 4] = [0; 4];
        tag.clone_from_slice(&lead_in[0..4]);

        if hex::encode(tag) != String::from("5444536d") {
            return Err(InvalidSegment());
        }

        let mut toc: [u8; 4] = [0; 4];
        toc.clone_from_slice(&lead_in[4..8]);

        // the Table of Contents is always in little endian format regardless if the rest of the segment
        // is in big endian
        let table_of_contents = u32::from_le_bytes(toc);

        let mut version: [u8; 4] = [0; 4];
        version.clone_from_slice(&lead_in[8..12]);

        let version_number = if table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            u32::from_be_bytes(version)
        } else {
            u32::from_le_bytes(version)
        };

        let mut offset: [u8; 8] = [0; 8];
        offset.clone_from_slice(&lead_in[12..20]);

        let next_segment_offset = if table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            u64::from_be_bytes(offset)
        } else {
            u64::from_le_bytes(offset)
        };

        let mut raw_offset: [u8; 8] = [0; 8];
        raw_offset.clone_from_slice(&lead_in[20..28]);

        let raw_data_offset = if table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            u64::from_be_bytes(raw_offset)
        } else {
            u64::from_le_bytes(raw_offset)
        };

        return Ok(LeadIn {
            tag,
            table_of_contents,
            version_number,
            next_segment_offset,
            raw_data_offset,
        });
    }
}

#[derive(Debug)]
/// `Metadata` represents the collection of metadata objects for a segment in the order in which they
/// were read
pub struct Metadata {
    number_of_objects: u32,
    objects: Vec<MetadataObject>,
}

#[derive(Debug)]
/// `MetadataObject` represents information that is not raw data associated with the segment. May
/// contain DAQmx raw data index, a standard index, or nothing at all.
pub struct MetadataObject {
    object_path: String,
    raw_data_index: Option<RawDataIndex>,
    daqmx_data_index: Option<DAQmxDataIndex>,
    properties: Vec<MetadataProperty>,
}

const DAQMX_FORMAT_SCALAR_IDENTIFIER: [u8; 4] = [69, 12, 00, 00];
const DAQMX_DIGITAL_LINE_SCALAR_IDENTIFIER: [u8; 4] = [69, 13, 00, 00];

impl Metadata {
    /// from_file accepts an open file and attempts to read metadata from the currently selected
    /// segment. Note that you must have read the segment's lead in information completely before
    /// attempting to use this function
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        file.read(&mut buf)?;

        let number_of_objects = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        let mut objects: Vec<MetadataObject> = vec![];

        for _ in 0..number_of_objects {
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

            let mut path = vec![0; length];
            file.read(&mut path)?;

            // all strings are UTF8 encoded in TDMS files, most prefixed by the length attribute
            // like above
            let object_path = match String::from_utf8(path) {
                Ok(n) => n,
                Err(_) => {
                    return Err(StringConversionError(String::from(
                        "unable to convert object path",
                    )))
                }
            };

            let mut buf: [u8; 4] = [0; 4];
            file.read(&mut buf)?;
            let mut raw_data_index: Option<RawDataIndex> = None;
            let mut daqmx_data_index: Option<DAQmxDataIndex> = None;

            let first_byte: u32 = match endianness {
                Little => u32::from_le_bytes(buf),
                Big => u32::from_be_bytes(buf),
            };

            // indicates format changing scaler
            if first_byte == 0x69120000 || first_byte == 0x00001269 {
                let index = DAQmxDataIndex::from_file(endianness, file, true)?;
                daqmx_data_index = Some(index);
                // indicates digital line scaler
            } else if first_byte == 0x69130000
                || first_byte == 0x0000126A
                || first_byte == 0x00001369
            {
                let index = DAQmxDataIndex::from_file(endianness, file, true)?;
                daqmx_data_index = Some(index);
            } else {
                if first_byte != 0xFFFFFFFF && first_byte != 0x0000000 {
                    raw_data_index = Some(RawDataIndex::from_file(endianness, file)?)
                }
            }

            file.read(&mut buf)?;
            let num_of_properties: u32 = match endianness {
                Little => u32::from_le_bytes(buf),
                Big => u32::from_be_bytes(buf),
            };

            // now we iterate through all the properties for the object
            let mut properties: Vec<MetadataProperty> = vec![];
            for _ in 0..num_of_properties {
                match MetadataProperty::from_file(endianness, file) {
                    Ok(p) => properties.push(p),
                    Err(e) => return Err(e),
                };
            }

            objects.push(MetadataObject {
                object_path,
                raw_data_index,
                daqmx_data_index,
                properties,
            });
        }

        return Ok(Metadata {
            number_of_objects,
            objects,
        });
    }
}

#[derive(Debug)]
pub struct RawDataIndex {
    data_type: TdmsDataType,
    array_dimension: u32, // should only ever be 1
    number_of_values: u64,
    number_of_bytes: Option<u64>, // only valid if data type is TDMS String
}

impl RawDataIndex {
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];

        // now we check the data type
        file.read(&mut buf)?;
        let data_type = match endianness {
            Big => i32::from_be_bytes(buf),
            Little => i32::from_le_bytes(buf),
        };

        let data_type = TdmsDataType::try_from(data_type)?;

        file.read(&mut buf)?;
        let array_dimension: u32 = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        let mut buf: [u8; 8] = [0; 8];
        file.read(&mut buf)?;
        let number_of_values = match endianness {
            Big => u64::from_be_bytes(buf),
            Little => u64::from_le_bytes(buf),
        };

        let number_of_bytes: Option<u64> = match data_type {
            TdmsDataType::String => {
                file.read(&mut buf)?;
                let num = match endianness {
                    Big => u64::from_be_bytes(buf),
                    Little => u64::from_le_bytes(buf),
                };

                Some(num)
            }
            _ => None,
        };

        return Ok(RawDataIndex {
            data_type,
            array_dimension,
            number_of_values,
            number_of_bytes,
        });
    }
}

#[derive(Debug)]
pub struct DAQmxDataIndex {
    data_type: TdmsDataType,
    array_dimension: u32, // should only ever be 1
    number_of_values: u64,
    format_changing_size: Option<u32>,
    format_changing_vec: Option<Vec<FormatChangingScaler>>,
    vec_size: u32,
    elements_in_vec: u32,
}

impl DAQmxDataIndex {
    pub fn from_file(
        endianness: Endianness,
        file: &mut File,
        is_format_changing: bool,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        file.read(&mut buf)?;

        let data_type = match endianness {
            Big => u32::from_be_bytes(buf),
            Little => u32::from_le_bytes(buf),
        };

        if data_type != 0xFFFFFFFF {
            return Err(InvalidDAQmxDataIndex());
        }

        file.read(&mut buf)?;
        let array_dimension = match endianness {
            Big => u32::from_be_bytes(buf),
            Little => u32::from_le_bytes(buf),
        };

        let mut buf: [u8; 8] = [0; 8];
        file.read(&mut buf)?;
        let number_of_values = match endianness {
            Big => u64::from_be_bytes(buf),
            Little => u64::from_le_bytes(buf),
        };

        let mut buf: [u8; 4] = [0; 4];

        let mut format_changing_size: Option<u32> = None;
        let mut format_changing_vec: Option<Vec<FormatChangingScaler>> = None;
        if is_format_changing {
            file.read(&mut buf)?;
            let changing_vec_size = match endianness {
                Big => u32::from_be_bytes(buf),
                Little => u32::from_le_bytes(buf),
            };

            let mut vec: Vec<FormatChangingScaler> = vec![];
            for _ in 0..changing_vec_size {
                vec.push(FormatChangingScaler::from_file(endianness, file)?)
            }
        }

        file.read(&mut buf)?;
        let vec_size = match endianness {
            Big => u32::from_be_bytes(buf),
            Little => u32::from_le_bytes(buf),
        };

        file.read(&mut buf)?;
        let elements_in_vec = match endianness {
            Big => u32::from_be_bytes(buf),
            Little => u32::from_le_bytes(buf),
        };

        return Ok(DAQmxDataIndex {
            data_type: TdmsDataType::DAQmxRawData,
            array_dimension,
            number_of_values,
            format_changing_size,
            format_changing_vec,
            vec_size,
            elements_in_vec,
        });
    }
}
#[derive(Debug)]
pub struct FormatChangingScaler {
    data_type: TdmsDataType,
    raw_buffer_index: u32,
    raw_byte_offset: u32,
    sample_format_bitmap: u32,
    scale_id: u32,
}

impl FormatChangingScaler {
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        file.read(&mut buf)?;

        let data_type = match endianness {
            Big => i32::from_be_bytes(buf),
            Little => i32::from_le_bytes(buf),
        };

        let data_type = TdmsDataType::try_from(data_type)?;

        file.read(&mut buf)?;
        let raw_buffer_index = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        file.read(&mut buf)?;
        let raw_byte_offset = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        file.read(&mut buf)?;
        let sample_format_bitmap = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        file.read(&mut buf)?;
        let scale_id = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        return Ok(FormatChangingScaler {
            data_type,
            raw_buffer_index,
            raw_byte_offset,
            sample_format_bitmap,
            scale_id,
        });
    }
}

#[derive(Debug)]
/// `MetadataProperty` is a key/value pair associated with a `MetadataObject`
pub struct MetadataProperty {
    name: String,
    data_type: TdmsDataType,
    value: TDMSValue,
}

impl MetadataProperty {
    /// from_file accepts an open file and attempts to read metadata properties from the currently
    /// selected segment and metadata object. Note that you must have read the metadata object's lead
    /// in information prior to using this function
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
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

        let mut name = vec![0; length];
        file.read(&mut name)?;

        // all strings are UTF8 encoded in TDMS files, most prefixed by the length attribute
        // like above
        let name = match String::from_utf8(name) {
            Ok(n) => n,
            Err(_) => {
                return Err(StringConversionError(String::from(
                    "unable to convert metadata property name",
                )))
            }
        };

        // now we check the data type
        file.read(&mut buf)?;
        let data_type = match endianness {
            Big => i32::from_be_bytes(buf),
            Little => i32::from_le_bytes(buf),
        };

        let data_type = TdmsDataType::try_from(data_type)?;
        let value = TDMSValue::from_file(endianness, data_type, file)?;

        return Ok(MetadataProperty {
            name,
            data_type,
            value,
        });
    }
}

#[derive(Debug)]
pub struct TDMSValue {
    data_type: TdmsDataType,
    endianness: Endianness,
    value: Option<Vec<u8>>,
}

impl TDMSValue {
    /// from_file accepts an open file and a data type and attempts to read the file, generating a
    /// value struct containing the actual value
    pub fn from_file(
        endianness: Endianness,
        data_type: TdmsDataType,
        file: &mut File,
    ) -> Result<Self, TdmsError> {
        match data_type {
            TdmsDataType::Void => {
                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: None,
                })
            }
            TdmsDataType::I8 => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::I16 => {
                let mut buf: [u8; 2] = [0; 2];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::I32 => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::I64 => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::U8 => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::U16 => {
                let mut buf: [u8; 2] = [0; 2];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::U32 => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::U64 => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::SingleFloat => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::DoubleFloat => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::ExtendedFloat => {
                let mut buf: [u8; 10] = [0; 10];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::SingleFloatWithUnit => {
                let mut buf: [u8; 4] = [0; 4];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::DoubleFloatWithUnit => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::ExtendedFloatWithUnit => {
                let mut buf: [u8; 10] = [0; 10];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
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

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(value),
                });
            }
            TdmsDataType::Boolean => {
                let mut buf: [u8; 1] = [0; 1];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::TimeStamp => {
                let mut buf: [u8; 16] = [0; 16];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            // there is little information on how to handle FixedPoint types, for
            // now we'll store them as a 64 bit integer and hope that will be enough
            TdmsDataType::FixedPoint => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::ComplexSingleFloat => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::ComplexDoubleFloat => {
                let mut buf: [u8; 16] = [0; 16];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
            TdmsDataType::DAQmxRawData => {
                let mut buf: [u8; 8] = [0; 8];
                file.read(&mut buf)?;

                return Ok(TDMSValue {
                    data_type,
                    endianness,
                    value: Some(buf.to_vec()),
                });
            }
        }
    }
}

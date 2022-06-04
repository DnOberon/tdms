use crate::{
    Big, General, InvalidDAQmxDataIndex, InvalidSegment, Little, StringConversionError, TDMSValue,
    TdmsDataType, TdmsError,
};
use std::fs::File;
use std::io::{Read, Seek};

/// These are bitmasks for the Table of Contents byte.
const K_TOC_META_DATA: u32 = 1 << 1;
const K_TOC_NEW_OBJ_LIST: u32 = 1 << 2;
const K_TOC_RAW_DATA: u32 = 1 << 3;
const K_TOC_INTERLEAVED_DATA: u32 = 1 << 5;
const K_TOC_BIG_ENDIAN: u32 = 1 << 6;
const K_TOC_DAQMX_RAW_DATA: u32 = 1 << 7;

/// Ease of use enum for determining how to read numerical values.
#[derive(Clone, Copy, Debug)]
pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug)]
/// `Segment` represents an entire TDMS File Segment and potentially its raw data.
pub struct Segment {
    pub lead_in: LeadIn,
    // TODO: remove Option when actually ready to parse
    pub metadata: Option<Metadata>,
    // TODO: remove Option when actually ready to parse
    pub raw_data: Option<Vec<u8>>,
    pub start_pos: u64,
    pub end_pos: u64,
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
    pub tag: [u8; 4],
    pub table_of_contents: u32,
    pub version_number: u32,
    pub next_segment_offset: u64,
    pub raw_data_offset: u64,
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
    pub number_of_objects: u32,
    pub objects: Vec<MetadataObject>,
}

#[derive(Debug)]
/// `MetadataObject` represents information that is not raw data associated with the segment. May
/// contain DAQmx raw data index, a standard index, or nothing at all.
pub struct MetadataObject {
    pub object_path: String,
    pub raw_data_index: Option<RawDataIndex>,
    pub daqmx_data_index: Option<DAQmxDataIndex>,
    pub properties: Vec<MetadataProperty>,
}

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
    pub data_type: TdmsDataType,
    pub array_dimension: u32, // should only ever be 1
    pub number_of_values: u64,
    pub number_of_bytes: Option<u64>, // only valid if data type is TDMS String
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
    pub data_type: TdmsDataType,
    pub array_dimension: u32, // should only ever be 1
    pub number_of_values: u64,
    pub format_changing_size: Option<u32>,
    pub format_changing_vec: Option<Vec<FormatChangingScaler>>,
    pub vec_size: u32,
    pub elements_in_vec: u32,
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

        let format_changing_size: Option<u32> = None;
        let format_changing_vec: Option<Vec<FormatChangingScaler>> = None;
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
    pub data_type: TdmsDataType,
    pub raw_buffer_index: u32,
    pub raw_byte_offset: u32,
    pub sample_format_bitmap: u32,
    pub scale_id: u32,
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
    pub name: String,
    pub data_type: TdmsDataType,
    pub value: TDMSValue,
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
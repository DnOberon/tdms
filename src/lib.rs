use std::any::Any;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::num::TryFromIntError;
use std::path::Path;
use std::string::FromUtf8Error;
use std::{fs, io};

mod error;
use crate::Endianness::{Big, Little};
use crate::TdmsError::{General, InvalidSegment, StringConversionError, UnsupportedVersion};
pub use error::TdmsError;

#[cfg(test)]
mod tests;

/// bitmasks for the Table of Contents byte
const K_TOC_META_DATA: u32 = 1 << 1;
const K_TOC_NEW_OBJ_LIST: u32 = 1 << 2;
const K_TOC_RAW_DATA: u32 = 1 << 3;
const K_TOC_INTERLEAVED_DATA: u32 = 1 << 5;
const K_TOC_BIG_ENDIAN: u32 = 1 << 6;
const K_TOC_DAQMX_RAW_DATA: u32 = 1 << 7;

/// datatype for matching to the data_type bytes after read
#[derive(Debug)]
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

pub enum Endianness {
    Little,
    Big,
}

#[derive(Debug)]
pub struct TDMSFile {
    segments: Vec<Segment>,
}

impl TDMSFile {
    pub fn from_path(path: &str, metadata_only: bool) -> Result<Self, TdmsError> {
        let metadata = fs::metadata(Path::new(path))?;
        let mut file = File::open(Path::new(path))?;
        let mut segments: Vec<Segment> = vec![];

        loop {
            let segment = Segment::new(&mut file)?;

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
    /// New expects a file who's cursor position is at the start of a new TDMS segment
    /// you will see an InvalidSegment error return if the file position isn't correct as the first
    /// byte read will not be the correct tag for a segment
    pub fn new(file: &mut File) -> Result<Self, TdmsError> {
        let start_pos = file.stream_position()?;
        let mut lead_in = [0; 28];

        file.read(&mut lead_in[..])?;

        let lead_in = LeadIn::from_bytes(&lead_in)?;

        // calculate the end position by taking the start and adding the offset plus lead in bytes
        let end_pos = lead_in.next_segment_offset + 28 + start_pos;

        return Ok(Segment {
            lead_in,
            metadata: None,
            raw_data: None,
            start_pos,
            /// lead in plus offset
            end_pos,
        });
    }

    /// the following function is not accurate unless the lead in portion of the segment has been read
    pub fn endianess(&self) -> Endianness {
        return if self.lead_in.table_of_contents & K_TOC_BIG_ENDIAN != 1 {
            Little
        } else {
            Big
        };
    }

    /// the following function is not accurate unless the lead in portion of the segment has been read
    pub fn interleaved_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_INTERLEAVED_DATA == 1;
    }

    /// the following function is not accurate unless the lead in portion of the segment has been read
    pub fn daqmx_raw_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_DAQMX_RAW_DATA == 1;
    }
}

#[derive(Debug)]
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

        let version_number = if table_of_contents & K_TOC_BIG_ENDIAN == 1 {
            u32::from_be_bytes(version)
        } else {
            u32::from_le_bytes(version)
        };

        if version_number != 4713 {
            return Err(UnsupportedVersion());
        }

        let mut offset: [u8; 8] = [0; 8];
        offset.clone_from_slice(&lead_in[12..20]);

        let next_segment_offset = if table_of_contents & K_TOC_BIG_ENDIAN == 1 {
            u64::from_be_bytes(offset)
        } else {
            u64::from_le_bytes(offset)
        };

        let mut raw_offset: [u8; 8] = [0; 8];
        raw_offset.clone_from_slice(&lead_in[20..28]);

        let raw_data_offset = if table_of_contents & K_TOC_BIG_ENDIAN == 1 {
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
pub struct Metadata {
    number_of_objects: u32,
    objects: Vec<MetadataObject>,
}

#[derive(Debug)]
pub struct MetadataObject {
    object_path: String,
    raw_data_index: Vec<u8>,
    properties: Vec<MetadataProperty>,
}

impl Metadata {
    // we must read from file because the length of the objects might be variable
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        file.read(&mut buf)?;

        let number_of_objects = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

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

            let name = match String::from_utf8(path) {
                Ok(n) => n,
                Err(_) => return Err(StringConversionError()),
            };
        }

        return Err(General(String::from("not implemented")));
    }
}

#[derive(Debug)]
pub struct MetadataProperty {
    name: String,
    data_type: TdmsDataType,
    value: Vec<u8>,
}

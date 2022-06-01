//! A Rust library for reading LabView TDMS files.
//!
//! More information about the TDMS file format can be found here: <https://www.ni.com/en-us/support/documentation/supplemental/07/tdms-file-format-internal-structure.html>
use std::any::Any;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::num::TryFromIntError;
use std::path::Path;
use std::string::FromUtf8Error;
use std::{fs, io};

mod error;
use crate::Endianness::{Big, Little};
use crate::TdmsError::{
    General, InvalidSegment, NotImplemented, StringConversionError, UnsupportedVersion,
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

/// Ease of use enum for determining how to read numerical values.
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

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn endianess(&self) -> Endianness {
        return if self.lead_in.table_of_contents & K_TOC_BIG_ENDIAN != 1 {
            Little
        } else {
            Big
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
    raw_data_index: Vec<u8>,
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
                Err(_) => return Err(StringConversionError()),
            };
        }

        let mut buf: [u8; 4] = [0; 4];
        file.read(&mut buf)?;

        if buf == DAQMX_FORMAT_SCALAR_IDENTIFIER {
            // TODO: implement
            return Err(NotImplemented);
        } else if buf == DAQMX_DIGITAL_LINE_SCALAR_IDENTIFIER {
            // TODO: implement
            return Err(NotImplemented);
        } else {
            let raw_data_index: u32 = match endianness {
                Little => u32::from_le_bytes(buf),
                Big => u32::from_be_bytes(buf),
            };

            if raw_data_index != 0xFFFFFFFF {
                // TODO: implement
                return Err(NotImplemented);
            }
        }

        return Err(General(String::from("not implemented")));
    }
}

#[derive(Debug)]
/// `MetadataProperty` is a key/value pair associated with a `MetadataObject`
pub struct MetadataProperty {
    name: String,
    data_type: TdmsDataType,
    value: Vec<u8>,
}

impl MetadataProperty {
    /// from_file accepts an open file and attempts to read metadata properties from the currently
    /// selected segment and metadata object. Note that you must have read the metadata object's lead
    /// in information prior to using this function
    pub fn from_file(endianness: Endianness, file: &mut File) -> Result<Self, TdmsError> {
        return Err(NotImplemented);
    }
}

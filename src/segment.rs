use crate::data_type::{TDMSValue, TdmsDataType};
use crate::{to_i32, to_u32, to_u64};
use crate::{
    Big, General, InvalidDAQmxDataIndex, InvalidSegment, Little, StringConversionError, TdmsError,
};
use indexmap::{indexmap, IndexMap};
use std::io::{Read, Seek};

/// These are bitmasks for the Table of Contents byte.
const K_TOC_META_DATA: u32 = 1 << 1;
// this flag represents a segment who's channel list/order has been changed from the previous segments
// and therefore a new order for processing the raw data must be followed
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

#[derive(Debug, Clone)]
/// `Segment` represents an entire TDMS File Segment and potentially its raw data.
pub struct Segment {
    pub lead_in: LeadIn,
    pub metadata: Option<Metadata>,
    pub start_pos: u64,
    pub end_pos: u64,
    pub groups: IndexMap<GroupPath, Option<IndexMap<ChannelPath, Channel>>>,
    pub chunk_size: u64,
}

/// GroupPath is a simple alias to allow our function signatures to be more telling
pub type GroupPath = String;
/// ChannelPath is a simple alias to allow our function signatures to be more telling
pub type ChannelPath = String;

#[derive(Clone, Debug)]
pub struct Channel {
    pub full_path: String,
    pub group_path: String,
    pub path: String,
    pub data_type: TdmsDataType,
    pub raw_data_index: Option<RawDataIndex>,
    pub daqmx_data_index: Option<DAQmxDataIndex>,
    pub properties: Vec<MetadataProperty>,
    pub chunk_positions: Vec<ChannelPositions>,
    pub interleaved_offset: u64,
}

#[derive(Clone, Debug, Copy)]
pub struct ChannelPositions(pub u64, pub u64);

impl Segment {
    /// `new` expects a reader who's cursor position is at the start of a new TDMS segment.
    /// You will see an InvalidSegment error return if the reader position isn't correct as the first
    /// byte read will not be the correct tag for a segment. The segment will hold on to the original
    /// reader in order to be able to return data not read into memory
    pub fn new<R: Read + Seek>(
        r: &mut R,
        previous_segment: Option<&Segment>,
    ) -> Result<Self, TdmsError> {
        let segment_start_pos = r.stream_position()?;
        let mut lead_in = [0; 28];

        r.read(&mut lead_in[..])?;

        let lead_in = LeadIn::from_bytes(&lead_in)?;

        // calculate the end position by taking the start and adding the offset plus lead in bytes
        let segment_end_pos = lead_in.next_segment_offset + 28 + segment_start_pos;

        let endianness = if lead_in.table_of_contents & K_TOC_BIG_ENDIAN != 0 {
            Big
        } else {
            Little
        };

        let mut metadata: Option<Metadata> = None;
        if lead_in.table_of_contents & K_TOC_META_DATA != 0 {
            metadata = Some(Metadata::from_reader(endianness, r)?);
        }

        // if we have have metadata, load up group and channel list for the segment - I debated
        // somehow building this list dynamically as we read the file but honestly the performance
        // hit according to benches was minimal and this makes a cleaner set of function boundaries
        // and lets us get away from passing in mutable state all over the place
        let mut groups: IndexMap<GroupPath, Option<IndexMap<ChannelPath, Channel>>> =
            IndexMap::<GroupPath, Option<IndexMap<ChannelPath, Channel>>>::new();

        // this variable will tell us where we're at in the raw data of the channel, allowing us to
        // set start and end thresholds of the channel's data itself
        let mut data_pos: u64 = segment_start_pos + lead_in.raw_data_offset;
        let mut interleaved_total_size: u64 = 0;
        let mut chunk_size: u64 = 0;

        match &mut metadata {
            Some(metadata) => {
                for obj in &mut metadata.objects {
                    let path = obj.object_path.clone();
                    let paths: Vec<&str> = path.split("/").collect();
                    let mut data_type: TdmsDataType = TdmsDataType::Void;

                    if previous_segment.is_some()
                        && obj.raw_data_index.is_none()
                        && obj.daqmx_data_index.is_none()
                    {
                        match previous_segment
                            .unwrap()
                            .get_channel(rem_quotes(paths[1]), rem_quotes(paths[2]))
                        {
                            None => {}
                            Some(c) => {
                                obj.raw_data_index = match &c.raw_data_index {
                                    None => None,
                                    Some(r) => Some(r.clone()),
                                };

                                obj.daqmx_data_index = match &c.daqmx_data_index {
                                    None => None,
                                    Some(d) => Some(d.clone()),
                                }
                            }
                        }
                    }

                    match &obj.raw_data_index {
                        None => {}
                        Some(index) => data_type = index.data_type,
                    }

                    match &obj.daqmx_data_index {
                        None => {}
                        Some(index) => data_type = index.data_type,
                    }

                    // add to the total interleaved size so we can calculate the offset later if needed
                    interleaved_total_size += TdmsDataType::get_size(data_type) as u64;

                    if paths.len() >= 2 && paths[1] != "" {
                        if !groups.contains_key(rem_quotes(paths[1])) {
                            let _ = groups.insert(rem_quotes(paths[1]).to_string(), None);
                        }
                    }

                    if paths.len() >= 3 && paths[2] != "" {
                        let map = groups.get_mut(rem_quotes(paths[1]));
                        let start_pos = data_pos.clone();
                        let mut end_pos = 0;

                        let raw_data_index = match &obj.raw_data_index {
                            Some(index) => {
                                let type_size = TdmsDataType::get_size(index.data_type);

                                // if not interleaved, the end threshold comes at chunk end
                                if lead_in.table_of_contents & K_TOC_INTERLEAVED_DATA == 0 {
                                    if index.data_type == TdmsDataType::String
                                        && index.number_of_bytes.is_some()
                                    {
                                        // our end position for a string channel should be the end of
                                        // the array of uint32s representing the index
                                        end_pos = data_pos + index.number_of_values * 4;

                                        // but we still need to iterate the main position to the end
                                        // of the chunk
                                        data_pos = data_pos + index.number_of_bytes.unwrap();
                                        chunk_size += index.number_of_bytes.unwrap();
                                    } else {
                                        data_pos = data_pos
                                            + (type_size as u64
                                                * index.array_dimension as u64
                                                * index.number_of_values);

                                        end_pos = data_pos.clone();
                                        chunk_size += type_size as u64
                                            * index.array_dimension as u64
                                            * index.number_of_values
                                    }
                                }

                                Some(index.clone())
                            }
                            None => None,
                        };

                        let daqmx_data_index = match &obj.daqmx_data_index {
                            Some(index) => Some(index.clone()),
                            None => None,
                        };

                        let channel = Channel {
                            full_path: obj.object_path.clone(),
                            group_path: rem_quotes(paths[1]).to_string(),
                            path: rem_quotes(paths[2]).to_string(),
                            data_type,
                            raw_data_index,
                            daqmx_data_index,
                            properties: obj.properties.clone(),
                            chunk_positions: vec![ChannelPositions(start_pos, end_pos)],
                            // this will be calculated later as we need all the channels information
                            // prior to calculating this offset
                            interleaved_offset: 0,
                        };

                        match map {
                            Some(map) => {
                                match map {
                                    Some(map) => {
                                        map.insert(rem_quotes(paths[2]).to_string(), channel);
                                    }
                                    None => {
                                        groups.insert(
                                    rem_quotes(paths[1]).to_string(),
                                    Some(indexmap! {rem_quotes(paths[2]).to_string() => channel}),
                                );
                                    }
                                }
                            }
                            None => (),
                        }
                    }
                }
            }
            _ => (),
        }

        if lead_in.table_of_contents & K_TOC_INTERLEAVED_DATA != 0 {
            for (_, channels) in groups.iter_mut() {
                match channels {
                    None => continue,
                    Some(channels) => {
                        for (_, channel) in channels.iter_mut() {
                            let size = TdmsDataType::get_size(channel.data_type);

                            // offset tells the iterator how many bytes to move to the next value
                            channel.interleaved_offset = interleaved_total_size - size as u64;
                            // update the end_pos_threshold  now that we have an idea of setup
                            match channel.chunk_positions.get_mut(0) {
                                None => (),
                                Some(positions) => {
                                    positions.1 = chunk_size - channel.interleaved_offset
                                        + interleaved_total_size
                                }
                            }

                            interleaved_total_size += size as u64;
                        }
                    }
                }
            }
        }

        // now we need to iterate yet again in order to build the start/end positions of each chunk
        for (_, channels) in groups.iter_mut() {
            match channels {
                None => continue,
                Some(channels) => {
                    for (_, channel) in channels.iter_mut() {
                        if channel.data_type == TdmsDataType::DAQmxRawData {
                            continue;
                        }

                        let mut i = 0;
                        loop {
                            let ChannelPositions(prev_start, prev_end) =
                                match channel.chunk_positions.get(i) {
                                    None => {
                                        return Err(General(String::from(
                                            "unable to fetch previous chunk positions",
                                        )))
                                    }
                                    Some(p) => p,
                                };

                            let new_start = prev_start + chunk_size;
                            let mut new_end = prev_end + chunk_size;

                            if new_start > segment_end_pos {
                                break;
                            }

                            if new_end > segment_end_pos {
                                new_end = segment_end_pos;
                            }

                            channel
                                .chunk_positions
                                .push(ChannelPositions(new_start, new_end));
                            i += 1;
                        }
                    }
                }
            }
        }

        //

        return Ok(Segment {
            lead_in,
            metadata,
            start_pos: segment_start_pos,
            // lead in plus offset
            end_pos: segment_end_pos,
            groups,
            chunk_size,
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
    pub fn has_interleaved_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_INTERLEAVED_DATA != 0;
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn has_daqmx_raw_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_DAQMX_RAW_DATA != 0;
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn has_raw_data(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_RAW_DATA != 0;
    }

    /// this function is not accurate unless the lead in portion of the segment has been read
    pub fn has_new_obj_list(&self) -> bool {
        return self.lead_in.table_of_contents & K_TOC_NEW_OBJ_LIST != 0;
    }

    pub fn get_channel_mut(&mut self, group_path: &str, path: &str) -> Option<&mut Channel> {
        let group = match self.groups.get_mut(group_path) {
            None => return None,
            Some(g) => g,
        };

        let channels = match group {
            None => return None,
            Some(c) => c,
        };

        return channels.get_mut(path);
    }

    pub fn get_channel(&self, group_path: &str, path: &str) -> Option<&Channel> {
        let group = match self.groups.get(group_path) {
            None => return None,
            Some(g) => g,
        };

        let channels = match group {
            None => return None,
            Some(c) => c,
        };

        return channels.get(path);
    }
}

#[derive(Debug, Clone)]
/// `LeadIn` represents the 28 bytes representing the lead in to a TDMS Segment.
pub struct LeadIn {
    pub tag: [u8; 4],
    pub table_of_contents: u32,
    pub version_number: u32,
    pub next_segment_offset: u64,
    pub raw_data_offset: u64,
}

impl LeadIn {
    /// `from_bytes` accepts a 28 byte array which represents the lead-in to a segment. This is hardcoded
    /// as there are no dynamic lengths in this portion of a segment
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
            u64::from_be_bytes(raw_offset) + 28
        } else {
            u64::from_le_bytes(raw_offset) + 28
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

#[derive(Debug, Clone)]
/// `Metadata` represents the collection of metadata objects for a segment in the order in which they
/// were read
pub struct Metadata {
    pub number_of_objects: u32,
    pub objects: Vec<MetadataObject>,
}

#[derive(Debug, Clone)]
/// `MetadataObject` represents information that is not raw data associated with the segment. May
/// contain DAQmx raw data index, a standard index, or nothing at all.
pub struct MetadataObject {
    pub object_path: String,
    pub raw_data_index: Option<RawDataIndex>,
    pub daqmx_data_index: Option<DAQmxDataIndex>,
    pub properties: Vec<MetadataProperty>,
}

impl Metadata {
    /// from_reader accepts an open reader and attempts to read metadata from the currently selected
    /// segment. Note that you must have read the segment's lead in information completely before
    /// attempting to use this function
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        r: &mut R,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        r.read(&mut buf)?;

        let number_of_objects = match endianness {
            Little => u32::from_le_bytes(buf),
            Big => u32::from_be_bytes(buf),
        };

        let mut objects: Vec<MetadataObject> = vec![];

        for _ in 0..number_of_objects {
            let mut buf: [u8; 4] = [0; 4];
            r.read(&mut buf)?;

            let length: u32 = to_u32!(buf, endianness);

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
            r.read_exact(&mut path)?;

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
            r.read_exact(&mut buf)?;
            let mut raw_data_index: Option<RawDataIndex> = None;
            let mut daqmx_data_index: Option<DAQmxDataIndex> = None;

            let first_byte: u32 = to_u32!(buf, endianness);

            // indicates format changing scaler
            if first_byte == 0x69120000 || first_byte == 0x00001269 {
                let index = DAQmxDataIndex::from_reader(endianness, r, true)?;
                daqmx_data_index = Some(index);
                // indicates digital line scaler
            } else if first_byte == 0x69130000
                || first_byte == 0x0000126A
                || first_byte == 0x00001369
            {
                let index = DAQmxDataIndex::from_reader(endianness, r, false)?;
                daqmx_data_index = Some(index);
            } else {
                if first_byte != 0xFFFFFFFF && first_byte != 0x0000000 {
                    raw_data_index = Some(RawDataIndex::from_reader(endianness, r)?)
                }
            }

            r.read_exact(&mut buf)?;
            let num_of_properties: u32 = to_u32!(buf, endianness);

            // now we iterate through all the properties for the object
            let mut properties: Vec<MetadataProperty> = vec![];
            for _ in 0..num_of_properties {
                match MetadataProperty::from_reader(endianness, r) {
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

#[derive(Debug, Clone)]
pub struct RawDataIndex {
    pub data_type: TdmsDataType,
    pub array_dimension: u32, // should only ever be 1
    pub number_of_values: u64,
    pub number_of_bytes: Option<u64>, // only valid if data type is TDMS String
}

impl RawDataIndex {
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        r: &mut R,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];

        // now we check the data type
        r.read_exact(&mut buf)?;
        let data_type = to_i32!(buf, endianness);

        let data_type = TdmsDataType::try_from(data_type)?;

        r.read_exact(&mut buf)?;
        let array_dimension: u32 = to_u32!(buf, endianness);

        let mut buf: [u8; 8] = [0; 8];
        r.read_exact(&mut buf)?;
        let number_of_values = to_u64!(buf, endianness);

        let number_of_bytes: Option<u64> = match data_type {
            TdmsDataType::String => {
                r.read_exact(&mut buf)?;
                let num = to_u64!(buf, endianness);

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

#[derive(Debug, Clone)]
pub struct DAQmxDataIndex {
    pub data_type: TdmsDataType,
    pub array_dimension: u32, // should only ever be 1
    pub number_of_values: u64,
    pub format_changing_size: Option<u32>,
    pub format_changing_vec: Option<Vec<FormatChangingScaler>>,
    pub buffer_vec_size: u32,
    pub buffers: Vec<u32>,
}

impl DAQmxDataIndex {
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        r: &mut R,
        is_format_changing: bool,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;

        let data_type = to_u32!(buf, endianness);

        if data_type != 0xFFFFFFFF {
            return Err(InvalidDAQmxDataIndex());
        }

        r.read_exact(&mut buf)?;
        let array_dimension = to_u32!(buf, endianness);

        let mut buf: [u8; 8] = [0; 8];
        r.read_exact(&mut buf)?;
        let number_of_values = to_u64!(buf, endianness);

        let mut buf: [u8; 4] = [0; 4];

        let format_changing_size: Option<u32> = None;
        let format_changing_vec: Option<Vec<FormatChangingScaler>> = None;
        if is_format_changing {
            r.read_exact(&mut buf)?;
            let changing_vec_size = to_u32!(buf, endianness);

            let mut vec: Vec<FormatChangingScaler> = vec![];
            for _ in 0..changing_vec_size {
                vec.push(FormatChangingScaler::from_reader(endianness, r)?)
            }
        }

        r.read_exact(&mut buf)?;
        let buffer_vec_size = to_u32!(buf, endianness);

        let mut buffers: Vec<u32> = vec![];

        for _ in 0..buffer_vec_size {
            r.read_exact(&mut buf)?;
            let elements = to_u32!(buf, endianness);

            buffers.push(elements);
        }

        return Ok(DAQmxDataIndex {
            data_type: TdmsDataType::DAQmxRawData,
            array_dimension,
            number_of_values,
            format_changing_size,
            format_changing_vec,
            buffer_vec_size,
            buffers,
        });
    }
}

#[derive(Debug, Clone)]
pub struct FormatChangingScaler {
    pub data_type: TdmsDataType,
    pub raw_buffer_index: u32,
    pub raw_byte_offset: u32,
    pub sample_format_bitmap: u32,
    pub scale_id: u32,
}

impl FormatChangingScaler {
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        r: &mut R,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;

        let data_type = to_i32!(buf, endianness);

        let data_type = TdmsDataType::try_from(data_type)?;

        r.read_exact(&mut buf)?;
        let raw_buffer_index = to_u32!(buf, endianness);

        r.read_exact(&mut buf)?;
        let raw_byte_offset = to_u32!(buf, endianness);

        r.read_exact(&mut buf)?;
        let sample_format_bitmap = to_u32!(buf, endianness);

        r.read_exact(&mut buf)?;
        let scale_id = to_u32!(buf, endianness);

        return Ok(FormatChangingScaler {
            data_type,
            raw_buffer_index,
            raw_byte_offset,
            sample_format_bitmap,
            scale_id,
        });
    }
}

#[derive(Debug, Clone)]
/// `MetadataProperty` is a key/value pair associated with a `MetadataObject`
pub struct MetadataProperty {
    pub name: String,
    pub data_type: TdmsDataType,
    pub value: TDMSValue,
}

impl MetadataProperty {
    /// from_reader accepts an open reader and attempts to read metadata properties from the currently
    /// selected segment and metadata object. Note that you must have read the metadata object's lead
    /// in information prior to using this function
    pub fn from_reader<R: Read + Seek>(
        endianness: Endianness,
        r: &mut R,
    ) -> Result<Self, TdmsError> {
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;

        let length: u32 = to_u32!(buf, endianness);

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
        r.read_exact(&mut name)?;

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
        r.read_exact(&mut buf)?;
        let data_type = to_i32!(buf, endianness);

        let data_type = TdmsDataType::try_from(data_type)?;
        let value = TDMSValue::from_reader(endianness, data_type, r)?;

        return Ok(MetadataProperty {
            name,
            data_type,
            value,
        });
    }
}

fn rem_quotes(value: &str) -> &str {
    let mut original = value.chars();
    let mut chars = value.clone().chars().peekable();

    match chars.peek() {
        None => (),
        Some(first) => {
            if first.to_string() == "'" {
                original.next();
            }
        }
    }

    let mut reversed = chars.rev().peekable();
    match reversed.peek() {
        None => (),
        Some(last) => {
            if last.to_string() == "'" {
                original.next_back();
            }
        }
    }

    original.as_str()
}

#[macro_export]
macro_rules! to_u32 {
    ( $x:ident, $t:ident ) => {
        match $t {
            Little => u32::from_le_bytes($x),
            Big => u32::from_be_bytes($x),
        }
    };
}

#[macro_export]
macro_rules! to_i32 {
    ( $x:ident, $t:ident ) => {
        match $t {
            Little => i32::from_le_bytes($x),
            Big => i32::from_be_bytes($x),
        }
    };
}

#[macro_export]
macro_rules! to_u64 {
    ( $x:ident, $t:ident ) => {
        match $t {
            Little => u64::from_le_bytes($x),
            Big => u64::from_be_bytes($x),
        }
    };
}

#[macro_export]
macro_rules! to_f64 {
    ( $x:ident, $t:ident ) => {
        match $t {
            Little => f64::from_le_bytes($x),
            Big => f64::from_be_bytes($x),
        }
    };
}

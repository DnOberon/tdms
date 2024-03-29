//! A Rust library for reading LabVIEW TDMS files.
//! `tdms` is a LabVIEW TDMS file parser library written in Rust. This is meant to be a general purpose library for reading and performing any calculation work on data contained in those files.
//!
//! **Note:** This library is a work in progress. While I do not expect the current function signatures and library structure to change, you could experience difficulties due to early adoption.
//!
//! ### Current Features
//! - Read both standard and big endian encoded files
//! - Read files with DAQmx data and data indices
//! - Read all segments in file, along with their groups and channels (per segment only)
//! - Read all raw data contained in all segments in file (as a `Vec<u8>` only at the present time)
//! - Logging using the `log` api - users of the library must choose and initialize the implementation, such as `env-logger`
//!
//! ### Planned Features
//! - Iterators for each channel type, return native Rust values from encoded data channels
//! - DAQmx data channel iterator support
//! - Searching on string channels
//!
//!
//! ## Usage
//!
//! ```rust
//!extern crate tdms;
//!
//! use std::path::Path;
//! use tdms::data_type::TdmsDataType;
//! use tdms::TDMSFile;
//!
//! fn main() {
//!     // open and parse the TDMS file, passing in metadata false will mean the entire file is
//!     // read into memory, not just the metadata
//!     let file = match TDMSFile::from_path(Path::new("data/standard.tdms")) {
//!         Ok(f) => f,
//!         Err(e) => panic!("{:?}", e),
//!     };
//!
//!     // fetch groups
//!     let groups = file.groups();
//!
//!     for group in groups {
//!         // fetch an IndexSet of the group's channels
//!         let channels = file.channels(&group);
//!
//!         let mut i = 0;
//!         for (_, channel) in channels {
//!             // once you know the channel's full path (group + channel) you can ask for the full
//!             // channel object. In order to fetch a channel you must call the proper channel func
//!             // depending on your data type. Currently this feature is unimplemented but the method
//!             // of calling this is set down for future changes
//!             let full_channel = match channel.data_type {
//!                 // the returned full channel is an iterator over raw data
//!                 TdmsDataType::DoubleFloat(_) => file.channel_data_double_float(channel),
//!                 _ => {
//!                     panic!("{}", "channel for data type unimplemented")
//!                 }
//!             };
//!
//!             let mut full_channel_iterator = match full_channel {
//!                 Ok(i) => i,
//!                 Err(e) => {
//!                     panic!("{:?}", e)
//!                 }
//!             };
//!
//!             println!("{:?}", full_channel_iterator.count());
//!
//!             i += 1;
//!         }
//!     }
//! }
//!
//! ```
//!
//! More information about the TDMS file format can be found here: <https://www.ni.com/en-us/support/documentation/supplemental/07/tdms-file-format-internal-structure.html>
//!
//! ## Contributing
//! Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.
//!
//! Please make sure to update tests as appropriate.
//!
//! ## License
//! [MIT](https://choosealicense.com/licenses/mit/)
//!
use indexmap::{IndexMap, IndexSet};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

pub mod error;
use crate::channel_iter::ChannelDataIter;
use crate::data_type::TdmsTimestamp;
use crate::TdmsError::{
    General, InvalidDAQmxDataIndex, InvalidSegment, StringConversionError, UnknownDataType,
};
pub use error::TdmsError;
use segment::Endianness::{Big, Little};
use segment::{Channel, Endianness, Segment};

pub mod channel_iter;
pub mod data_type;
pub mod segment;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
/// `TDDMSFile` represents all `segments` of a TDMS file in the order in which they were read.
pub struct TDMSFile<'a> {
    pub segments: Vec<Segment>,
    path: &'a Path,
}

impl<'a> TDMSFile<'a> {
    /// `from_path` expects a path and whether or not to read only the metadata of each segment vs
    /// the entire file into working memory.
    pub fn from_path(path: &'a Path) -> Result<Self, TdmsError> {
        let metadata = fs::metadata(path)?;
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(4096, file);
        let mut segments: Vec<Segment> = vec![];
        let mut i = 0;

        loop {
            let previous_segment = if i == 0 { None } else { segments.get(i - 1) };
            let segment = Segment::new(&mut reader, previous_segment)?;

            if segment.end_pos == metadata.len() {
                segments.push(segment);
                break;
            }

            reader.seek(SeekFrom::Start(segment.end_pos))?;
            segments.push(segment);
            i += 1;
        }

        return Ok(TDMSFile { segments, path });
    }

    /// groups returns all possible groups throughout the file
    pub fn groups(&self) -> Vec<String> {
        let mut map: IndexSet<String> = IndexSet::new();

        for segment in &self.segments {
            for (group, _) in &segment.groups {
                map.insert(String::from(group));
            }
        }

        return Vec::from_iter(map);
    }

    pub fn channels(&self, group_path: &str) -> IndexMap<String, &Channel> {
        let mut map: IndexMap<String, &Channel> = IndexMap::new();

        for segment in &self.segments {
            let channel_map = match segment.groups.get(group_path) {
                Some(m) => m,
                None => &None,
            };

            let channel_map = match channel_map {
                None => continue,
                Some(m) => m,
            };

            for (channel_path, channel) in channel_map {
                map.insert(String::from(channel_path), channel);
            }
        }

        return map;
    }

    /// returns a channel who's type is the native rust type equivalent to TdmsDoubleFloat, in this
    /// case `f64` - the channel implements Iterator and using said iterator will let you move through
    /// the channel's raw data if any exists
    pub fn channel_data_double_float(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f64, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_single_float(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f32, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_complex_double_float(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f64, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_complex_single_float(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f32, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_double_float_unit(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f64, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_single_float_unit(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<f32, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_i8(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<i8, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_i16(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<i16, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_i32(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<i32, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_i64(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<i64, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_u8(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<u8, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_u16(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<u16, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_u32(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<u32, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_u64(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<u64, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_bool(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<bool, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_timestamp(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<TdmsTimestamp, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    pub fn channel_data_string(
        &self,
        channel: &'a Channel,
    ) -> Result<ChannelDataIter<String, File>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());
        let reader = BufReader::with_capacity(4096, File::open(self.path)?);

        return ChannelDataIter::new(vec, channel, reader);
    }

    fn load_segments(&self, group_path: &str, path: &str) -> Vec<&Segment> {
        let mut vec: Vec<&Segment> = vec![];
        let mut channel_in_segment: bool = false;

        for segment in &self.segments {
            match segment.groups.get(group_path) {
                None => {
                    if !segment.has_new_obj_list() && channel_in_segment {
                        vec.push(&segment)
                    } else {
                        channel_in_segment = false
                    }
                }
                Some(channels) => match channels {
                    None => {
                        if !segment.has_new_obj_list() && channel_in_segment {
                            vec.push(&segment)
                        } else {
                            channel_in_segment = false
                        }
                    }
                    Some(channels) => {
                        let channel = channels.get(path);

                        match channel {
                            None => {
                                if !segment.has_new_obj_list() && channel_in_segment {
                                    vec.push(&segment)
                                } else {
                                    channel_in_segment = false
                                }
                            }
                            Some(_) => {
                                vec.push(&segment);
                                channel_in_segment = true;
                            }
                        }
                    }
                },
            }
        }

        return vec;
    }
}

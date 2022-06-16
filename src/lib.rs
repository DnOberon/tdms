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
//!
//!
//! ## Usage
//!
//! ```rust
//! extern crate tdms;
//!
//! use std::path::Path;
//! use tdms::data_type::TdmsDataType;
//! use tdms::TDMSFile;
//!
//! fn main() {
//!     // open and parse the TDMS file, passing in metadata false will mean the entire file is
//!     // read into memory, not just the metadata
//!     let file = match TDMSFile::from_path(Path::new("data/standard.tdms"), false) {
//!         Ok(f) => f,
//!        Err(e) => panic!("{:?}", e),
//!     };
//!
//!     // fetch groups
//!     let groups = file.groups();
//!
//!     for group in groups {
//!        // fetch an IndexSet of the group's channels
//!         let channels = file.channels(&group);
//!
//!         for (channelPath,  channel) in channels {
//!             // once you know the channel's full path (group + channel) you can ask for the full
//!             // channel object. In order to fetch a channel you must call the proper channel func
//!             // depending on your data type. Currently this feature is unimplemented but the method
//!             // of calling this is set down for future changes
//!             let full_channel = match data_type { // the returned full channel is an iterator over raw data
//!                 TdmsDataType::DoubleFloat(_) => file.channel_data_double_float(channel),
//!                 _ => {
//!                     panic!("{}", "channel for data type unimplemented")
//!                 }
//!             };
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
use data_type::TdmsDataType;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub mod error;
use crate::channel::{Channel, ChannelDataIter};
use crate::TdmsError::{
    General, InvalidDAQmxDataIndex, InvalidSegment, StringConversionError, UnknownDataType,
};
pub use error::TdmsError;
use segment::Endianness::{Big, Little};
use segment::{Endianness, Segment};

pub mod channel;
pub mod data_type;
pub mod segment;
#[cfg(test)]
mod tests;

#[derive(Debug)]
/// `TDDMSFile` represents all `segments` of a TDMS file in the order in which they were read.
pub struct TDMSFile<R: Read + Seek> {
    pub segments: Vec<Segment>,
    reader: BufReader<R>,
}

impl TDMSFile<File> {
    /// `from_path` expects a path and whether or not to read only the metadata of each segment vs
    /// the entire file into working memory.
    pub fn from_path(path: &Path, metadata_only: bool) -> Result<Self, TdmsError> {
        let metadata = fs::metadata(path)?;
        let file = File::open(path)?;
        let mut reader = BufReader::with_capacity(4096, file);
        let mut segments: Vec<Segment> = vec![];

        loop {
            let segment = Segment::new(&mut reader, metadata_only)?;

            if segment.end_pos == metadata.len() {
                segments.push(segment);
                break;
            }

            reader.seek(SeekFrom::Start(segment.end_pos))?;
            segments.push(segment);
        }

        return Ok(TDMSFile { segments, reader });
    }
}

impl<R: Read + Seek> TDMSFile<R> {
    /// groups returns all possible groups throughout the file
    pub fn groups(&self) -> Vec<String> {
        let mut map: HashSet<String> = HashSet::new();

        for segment in &self.segments {
            for (group, _) in &segment.groups {
                map.insert(String::from(group));
            }
        }

        return Vec::from_iter(map);
    }

    pub fn channels(&self, group_path: &str) -> HashMap<String, &Channel> {
        let mut map: HashMap<String, &Channel> = HashMap::new();

        for segment in &self.segments {
            let channel_map = match segment.groups.get(group_path) {
                Some(m) => m,
                None => &None,
            };

            let channel_map = match channel_map {
                None => continue,
                Some(m) => m,
            };

            for (channelPath, channel) in channel_map {
                map.insert(String::from(channelPath), channel);
            }
        }

        return map;
    }

    /// returns a channel who's type is the native rust type equivalent to TdmsDoubleFloat, in this
    /// case `f64` - the channel implements Iterator and using said iterator will let you move through
    /// the channel's raw data if any exists
    pub fn channel_data_double_float(
        &self,
        channel: &Channel,
    ) -> Result<ChannelDataIter<R, f64>, TdmsError> {
        let vec = self.load_segments(channel.group_path.as_str(), channel.path.as_str());

        return ChannelDataIter::new(vec, channel.clone(), &self.reader);
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

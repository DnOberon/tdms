use crate::segment::{ChannelPath, GroupPath};
use crate::{General, Segment, TdmsError};
use std::fs::File;
use std::io::{BufReader, Read, Seek};

#[derive(Debug)]
pub struct Channel<'a, R: Read + Seek> {
    group_path: GroupPath,
    path: ChannelPath,
    segments: Vec<&'a Segment>,
    bytes_read: u64,
    current_segment: &'a Segment,
    current_segment_index: usize,
    reader: &'a BufReader<R>,
}

impl<'a, R: Read + Seek> Channel<'a, R> {
    pub fn new(
        segments: Vec<&'a Segment>,
        group_path: String,
        path: String,
        reader: &'a BufReader<R>,
    ) -> Result<Self, TdmsError> {
        if segments.len() <= 0 {
            return Err(General(String::from(
                "no segments provided for channel creation",
            )));
        }

        let current_segment = segments[0];

        return Ok(Channel {
            group_path,
            path,
            segments,
            bytes_read: 0,
            current_segment,
            current_segment_index: 0,
            reader,
        });
    }
}

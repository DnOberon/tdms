use crate::segment::{ChannelPath, DAQmxDataIndex, GroupPath, RawDataIndex};
use crate::{General, Segment, TdmsError};

#[derive(Debug, Clone)]
pub struct Channel<'a> {
    group_path: GroupPath,
    path: ChannelPath,
    segments: Vec<&'a Segment>,
    bytes_read: u64,
    current_segment: &'a Segment,
}

impl<'a> Channel<'a> {
    pub fn new(
        segments: Vec<&'a Segment>,
        group_path: String,
        path: String,
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
        });
    }
}

use crate::segment::{DAQmxDataIndex, Metadata, RawDataIndex};
use crate::{General, Segment, TdmsError};

#[derive(Debug, Clone)]
pub struct Channel<'a> {
    path: String,
    segments: Vec<&'a Segment>,
    bytes_read: u64,
    current_segment: &'a Segment,
    raw_data_index: &'a Option<RawDataIndex>,
    daqmx_data_index: &'a Option<DAQmxDataIndex>,
}

impl<'a> Channel<'a> {
    pub fn new(segments: Vec<&'a Segment>, path: String) -> Result<Self, TdmsError> {
        if segments.len() <= 0 {
            return Err(General(String::from(
                "no segments provided for channel creation",
            )));
        }

        let current_segment = segments[0];

        let mut raw_data_index: &Option<RawDataIndex> = &None;
        let mut daqmx_data_index: &Option<DAQmxDataIndex> = &None;

        match &current_segment.metadata {
            None => {}
            Some(metadata) => {
                for object in &metadata.objects {
                    raw_data_index = &object.raw_data_index;
                    daqmx_data_index = &object.daqmx_data_index
                }
            }
        }

        return Ok(Channel {
            path,
            segments,
            bytes_read: 0,
            current_segment,
            raw_data_index,
            daqmx_data_index,
        });
    }
}

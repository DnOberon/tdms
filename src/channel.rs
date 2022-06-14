use crate::segment::{ChannelPath, DAQmxDataIndex, GroupPath, MetadataProperty, RawDataIndex};
use crate::{General, Segment, TdmsDataType, TdmsError};
use std::io::{BufReader, Read, Seek};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct Channel {
    pub full_path: String,
    pub path: String,
    pub data_type: TdmsDataType,
    pub raw_data_index: Option<RawDataIndex>,
    pub daqmx_data_index: Option<DAQmxDataIndex>,
    pub properties: Vec<MetadataProperty>,
}

#[derive(Debug)]
pub struct ChannelData<'a, R: Read + Seek, T> {
    group_path: GroupPath,
    path: ChannelPath,
    segments: Vec<&'a Segment>,
    bytes_read: u64,
    current_segment: &'a Segment,
    current_segment_index: usize,
    reader: &'a BufReader<R>,
    _mask: PhantomData<T>,
}

impl<'a, R: Read + Seek, T> ChannelData<'a, R, T> {
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

        return Ok(ChannelData {
            group_path,
            path,
            segments,
            bytes_read: 0,
            current_segment,
            current_segment_index: 0,
            reader,
            _mask: Default::default(),
        });
    }
}

impl<'a, R: Read + Seek> Iterator for ChannelData<'a, R, f64> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

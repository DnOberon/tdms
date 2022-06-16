use crate::segment::Channel;
use crate::{General, Segment, TdmsError};
use std::io::{BufReader, Read, Seek, Take};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ChannelDataIter<'a, T, R: Read + Seek> {
    channel: Channel,
    segments: Vec<&'a Segment>,
    bytes_read: u64,
    current_segment: Segment,
    current_segment_index: usize,
    current_segment_data: Take<BufReader<R>>,
    _mask: PhantomData<T>,
}

impl<'a, T, R: Read + Seek> ChannelDataIter<'a, T, R> {
    pub fn new(
        segments: Vec<&'a Segment>,
        channel: Channel,
        reader: BufReader<R>,
    ) -> Result<Self, TdmsError> {
        if segments.len() <= 0 {
            return Err(General(String::from(
                "no segments provided for channel creation",
            )));
        }

        let mut current_segment = segments[0].clone();
        let current_segment_data = current_segment.raw_data_reader(reader);
        let current_segment_data = match current_segment_data {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        return Ok(ChannelDataIter {
            channel,
            segments,
            bytes_read: 0,
            current_segment,
            current_segment_index: 0,
            current_segment_data,
            _mask: Default::default(),
        });
    }
}

impl<'a, R: Read + Seek> Iterator for ChannelDataIter<'a, f64, R> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

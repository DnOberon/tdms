use crate::data_type::TdmsDataType;
use crate::segment::Channel;
use crate::TdmsError::{EndOfSegments, IntConversionError, NotImplemented, ReadError};
use crate::{Endianness, General, Segment, TdmsError};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ChannelDataIter<'a, T, R: Read + Seek> {
    channel: Channel,
    segments: Vec<&'a Segment>,
    reader: BufReader<R>,
    _mask: PhantomData<T>,
    pub error: Option<TdmsError>,
}

impl<'a, T, R: Read + Seek> ChannelDataIter<'a, T, R> {
    pub fn new(
        segments: Vec<&'a Segment>,
        channel: Channel,
        mut reader: BufReader<R>,
    ) -> Result<Self, TdmsError> {
        if segments.len() <= 0 {
            return Err(General(String::from(
                "no segments provided for channel creation",
            )));
        }

        let mut iter = ChannelDataIter {
            channel,
            segments,
            reader,
            _mask: Default::default(),
            error: None,
        };

        // set the reader to the first segment's start position so that the rest of the reader works
        // correctly
        match iter.segments.get(0) {
            None => {}
            Some(s) => {
                iter.reader.seek(SeekFrom::Start(s.start_pos))?;
            }
        }

        iter.reader_to_data_start();

        return Ok(iter);
    }

    /// segment_index_for_reader returns the current segment for the reader's current position
    fn current_segment_index(&mut self) -> usize {
        let stream_pos = match self.reader.stream_position() {
            Ok(p) => p,
            Err(_) => 0,
        };

        let mut index = 0;

        for (i, s) in self.segments.iter().enumerate() {
            if s.end_pos <= stream_pos {
                continue;
            }

            index = i
        }

        return index;
    }

    /// set_reader_to_raw moves the internal reader's pointer to the initial raw data value for this
    /// channel - used when iterating segments or at startup
    fn reader_to_data_start(&mut self) -> Option<TdmsError> {
        let index = self.current_segment_index();
        let current_segment = match self.segments.get(index) {
            None => return Some(EndOfSegments()),
            Some(s) => s,
        };

        // set to the current segment's raw data portion and set the file pointer to the correct
        // location
        match self.reader.seek(SeekFrom::Start(
            current_segment.start_pos + current_segment.lead_in.raw_data_offset,
        )) {
            Ok(_) => {}
            Err(e) => return Some(ReadError(e)),
        }

        // iterate through the channels in the current segment, moving the file pointer to the proper
        // starting location for this channel
        for (group_path, channels) in &current_segment.groups {
            match channels {
                Some(channels) => {
                    for (channel_path, current_channel) in channels {
                        // if we've reached our channel the pointer is in the right location
                        if group_path.as_str() == self.channel.group_path.as_str()
                            && channel_path.as_str() == self.channel.path.as_str()
                        {
                            break;
                        }

                        // if we're not our channel, move the pointer to the next channel (or next
                        // value if we're interleaved)
                        let size: usize = TdmsDataType::get_size(current_channel.data_type);

                        if current_channel.data_type == TdmsDataType::Void {
                            continue;
                        }

                        if current_channel.data_type == TdmsDataType::String {
                            return Some(NotImplemented(String::from(
                                "string channel type reading",
                            )));
                        }

                        match &current_channel.raw_data_index {
                            None => (),
                            Some(index) => {
                                if current_channel.data_type == TdmsDataType::String {
                                    let number_of_bytes =
                                        match i64::try_from(match index.number_of_bytes {
                                            Some(v) => v,
                                            None => 0,
                                        }) {
                                            Ok(v) => v,
                                            Err(e) => return Some(IntConversionError(e)),
                                        };

                                    if current_segment.has_interleaved_data() {
                                        return Some(NotImplemented(String::from(
                                            "interleaved data string reading",
                                        )));
                                    } else {
                                        match self.reader.seek(SeekFrom::Current(number_of_bytes)) {
                                            Ok(_) => {}
                                            Err(e) => return Some(ReadError(e)),
                                        };
                                    }
                                } else {
                                    // fairly safe type cast here, we know for a fact size will never
                                    // overflow a u64
                                    let number_of_bytes = match i64::try_from(
                                        size as u64
                                            * index.array_dimension as u64
                                            * index.number_of_values,
                                    ) {
                                        Ok(v) => v,
                                        Err(e) => return Some(IntConversionError(e)),
                                    };

                                    // interleaved means we only need to advance the pointer by one
                                    // value vs. the whole channel's data
                                    if current_segment.has_interleaved_data() {
                                        match self.reader.seek(SeekFrom::Current(size as i64)) {
                                            Ok(_) => {}
                                            Err(e) => return Some(ReadError(e)),
                                        };
                                    } else {
                                        match self.reader.seek(SeekFrom::Current(number_of_bytes)) {
                                            Ok(_) => {}
                                            Err(e) => return Some(ReadError(e)),
                                        };
                                    }
                                }
                            }
                        }

                        // TODO: implement daqmx data reading
                        match &current_channel.daqmx_data_index {
                            None => {}
                            Some(_) => {
                                return Some(NotImplemented(String::from("daqmx data channels")))
                            }
                        }
                    }
                }
                None => continue,
            }
        }

        let mut values_in_segment: u64 = 0;
        let size = TdmsDataType::get_size(self.channel.data_type);

        match &self.channel.raw_data_index {
            Some(index) => {
                values_in_segment =
                    // again, safe casts because we know the size won't overflow a u64
                    (size as u64 * index.array_dimension as u64 * index.number_of_values) / 8
            }
            None => (),
        }

        match &self.channel.daqmx_data_index {
            Some(_) => return Some(NotImplemented(String::from("daqmx data channels"))),
            None => (),
        }

        return None;
    }

    /// advance_reader_to_next moves the internal BufReader<R> to the next valid data value depending
    /// on data type, index, current pos. etc - this function also handles iterating to the next
    /// segment if necessary
    fn advance_reader_to_next(&mut self) -> () {}
}

impl<'a, R: Read + Seek> Iterator for ChannelDataIter<'a, f64, R> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let segment_index = self.current_segment_index();
        let current_segment = match self.segments.get(segment_index) {
            None => return None,
            Some(c) => c,
        };

        // to check the required byte size of this channel's data type, look
        // at data_types.rs and the TdmsDataType enum
        let mut buf: [u8; 8] = [0; 8];

        match self.reader.read_exact(&mut buf) {
            Ok(_) => (),
            Err(e) => {
                self.error = Some(ReadError(e));
                return None;
            }
        }

        let value = match current_segment.endianess() {
            Endianness::Little => Some(f64::from_le_bytes(buf)),
            Endianness::Big => Some(f64::from_be_bytes(buf)),
        };

        // advance to next value - this function handles interleaved iteration and moving to the
        // next segment
        self.advance_reader_to_next();

        return value;
    }
}

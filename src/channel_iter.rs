use crate::data_type::TdmsDataType;
use crate::segment::{Channel, ChannelPositions};
use crate::TdmsError::{ChannelDoesNotExist, EndOfSegments, GroupDoesNotExist};
use crate::{Endianness, General, Segment, TdmsError};
use std::cell::RefCell;
use std::io::{BufReader, ErrorKind, Read, Seek, SeekFrom};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ChannelDataIter<'a, T, R: Read + Seek> {
    channel: RefCell<&'a Channel>,
    segments: Vec<&'a Segment>,
    reader: BufReader<R>,
    _mask: PhantomData<T>,
}

impl<'a, T, R: Read + Seek> ChannelDataIter<'a, T, R> {
    pub fn new(
        segments: Vec<&'a Segment>,
        channel: &'a Channel,
        mut reader: BufReader<R>,
    ) -> Result<Self, TdmsError> {
        if segments.len() <= 0 {
            return Err(General(String::from(
                "no segments provided for channel creation",
            )));
        }

        // overwrite the passed in channel with the first channel in the segments
        let channel =
            match segments[0].get_channel(channel.group_path.as_str(), channel.path.as_str()) {
                None => channel,
                Some(c) => c,
            };

        let channel = RefCell::new(channel);

        let mut iter = ChannelDataIter {
            channel,
            segments,
            reader,
            _mask: Default::default(),
        };

        // set the reader to the first segment's start position so that the rest of the reader works
        // correctly
        match iter.segments.get(0) {
            None => {}
            Some(s) => {
                iter.reader.seek(SeekFrom::Start(s.start_pos))?;
            }
        }

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
            if stream_pos < s.end_pos {
                index = i;
                break;
            }
        }

        return index;
    }

    fn current_positions(&mut self) -> Result<ChannelPositions, TdmsError> {
        let stream_pos = self.reader.stream_position()?;

        for positions in self.channel.borrow().chunk_positions.iter() {
            if stream_pos >= positions.1 {
                continue;
            }

            return Ok(positions.clone());
        }

        let index = self.current_segment_index();

        let current_segment = match self.segments.get(index) {
            None => return Err(EndOfSegments()),
            Some(s) => s,
        };

        self.reader.seek(SeekFrom::Start(current_segment.end_pos))?;
        let current_segment = match self.segments.get(index + 1) {
            None => return Err(EndOfSegments()),
            Some(s) => s,
        };

        // we can error out here because if this is a new segment, but that segment doesn't
        // have the channels we want, we need to error out
        let channels = match current_segment
            .groups
            .get(&self.channel.borrow().group_path)
        {
            None => return Err(GroupDoesNotExist()),
            Some(g) => g,
        };

        let channel_map = match channels {
            None => return Err(ChannelDoesNotExist()),
            Some(c) => c,
        };

        let channel = match channel_map.get(&self.channel.borrow().path) {
            None => return Err(ChannelDoesNotExist()),
            Some(channel) => channel,
        };

        self.channel.swap(&RefCell::new(channel));

        return self.current_positions();
    }

    /// advance_reader_to_next moves the internal BufReader<R> to the next valid data value depending
    /// on data type, index, current pos. etc - this function also handles iterating to the next
    /// segment if necessary
    fn advance_reader_to_next(&mut self) -> Result<&Segment, TdmsError> {
        let index = self.current_segment_index();
        let ChannelPositions(start_pos, end_pos) = self.current_positions()?;

        let current_segment = match self.segments.get(index) {
            None => return Err(EndOfSegments()),
            Some(s) => s,
        };

        let stream_pos = self.reader.stream_position()?;

        // if we're not past data start, move us there first
        if stream_pos < current_segment.start_pos + current_segment.lead_in.raw_data_offset
            || stream_pos < start_pos
        {
            self.reader.seek(SeekFrom::Start(start_pos))?;
        }

        let stream_pos = self.reader.stream_position()?;
        // if we're past the channel's end pos for the segment, move to the end of segment and
        // recursively call this function - setting the new channel's raw index and calculating
        // start and end pos if needed
        if stream_pos >= end_pos {
            self.reader.seek(SeekFrom::Start(current_segment.end_pos))?;
            let current_segment = match self.segments.get(index + 1) {
                None => return Err(EndOfSegments()),
                Some(s) => s,
            };

            // we can error out here because if this is a new segment, but that segment doesn't
            // have the channels we want, we need to error out
            let channels = match current_segment
                .groups
                .get(&self.channel.borrow().group_path)
            {
                None => return Err(GroupDoesNotExist()),
                Some(g) => g,
            };

            let channel_map = match channels {
                None => return Err(ChannelDoesNotExist()),
                Some(c) => c,
            };

            let channel = match channel_map.get(&self.channel.borrow().path) {
                None => return Err(ChannelDoesNotExist()),
                Some(channel) => channel,
            };

            self.channel.swap(&RefCell::new(channel));

            return self.advance_reader_to_next();
        }

        // iterate by interleaved offset if interleaved data
        if current_segment.has_interleaved_data() {
            self.reader.seek(SeekFrom::Current(
                self.channel.borrow().interleaved_offset as i64,
            ))?;

            return self.advance_reader_to_next();
        }

        let stream_pos = self.reader.stream_position()?;

        if stream_pos < start_pos {
            self.reader.seek(SeekFrom::Start(start_pos))?;
        }

        let stream_pos = self.reader.stream_position()?;

        if stream_pos >= start_pos && stream_pos < end_pos {
            return Ok(current_segment);
        }

        return self.advance_reader_to_next();
    }
}

impl<'a, R: Read + Seek> Iterator for ChannelDataIter<'a, f64, R> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        // advance to next value - this function handles interleaved iteration and moving to the
        // next segment TODO: get a passed in logger and output to that logger channel
        let current_segment = self.advance_reader_to_next();
        let endianess = match current_segment {
            Err(e) => {
                match e {
                    EndOfSegments() => (),
                    _ => println!("error reading next value in channel: {:?}", e),
                }

                return None;
            }
            Ok(s) => s.endianess(),
        };

        // to check the required byte size of this channel's data type, look
        // at data_types.rs and the TdmsDataType enum
        let mut buf: [u8; 8] = [0; 8];

        match self.reader.read_exact(&mut buf) {
            Ok(_) => (),
            Err(e) => {
                match e.kind() {
                    ErrorKind::UnexpectedEof => {}
                    // TODO: bring in logger and print to  their log
                    _ => println!("error reading value from file ${:?}", e),
                }

                return None;
            }
        }

        let value = match endianess {
            Endianness::Little => Some(f64::from_le_bytes(buf)),
            Endianness::Big => Some(f64::from_be_bytes(buf)),
        };

        return value;
    }
}

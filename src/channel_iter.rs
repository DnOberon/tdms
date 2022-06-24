use crate::data_type::TdmsDataType;
use crate::segment::{Channel, ChannelPath};
use crate::TdmsError::{EndOfSegments, IntConversionError, NotImplemented, ReadError};
use crate::{Endianness, General, Segment, TdmsError};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ChannelDataIter<'a, T, R: Read + Seek> {
    channel: RefCell<&'a Channel>,
    segments: Vec<&'a Segment>,
    reader: BufReader<R>,
    _mask: PhantomData<T>,
    pub error: Option<TdmsError>,
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

        let channel = RefCell::new(channel);

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

    /// advance_reader_to_next moves the internal BufReader<R> to the next valid data value depending
    /// on data type, index, current pos. etc - this function also handles iterating to the next
    /// segment if necessary
    fn advance_reader_to_next(&mut self) -> Result<(), TdmsError> {
        let index = self.current_segment_index();

        let current_segment = match self.segments.get(index) {
            None => return Err(EndOfSegments()),
            Some(s) => s,
        };

        let stream_pos = self.reader.stream_position()?;

        // if we're not past data start, move us there first
        if stream_pos < current_segment.start_pos + current_segment.lead_in.raw_data_offset {
            self.reader
                .seek(SeekFrom::Start(self.channel.borrow().start_pos))?;
        }

        // if we're past the channel's end pos for the segment, move to the end of the segment and
        // recursively call this function - setting the new channel's raw index and calculating
        // start and end pos if needed
        if stream_pos >= self.channel.borrow().end_pos {
            self.reader.seek(SeekFrom::Start(current_segment.end_pos))?;

            // TODO: check new segment if has new obj list, if does, can ignore this
            // TODO: check new segment if has info on the current channel, if it does, check the data index if data index new, use to calculate start and end pos
            // TODO: if indexes are not new, use current channel's raw data index to calculate start and end pos
            // TODO: update the stored channel's start and end pos

            return self.advance_reader_to_next();
        }

        return Ok(());
    }
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

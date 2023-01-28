extern crate tdms;

use std::path::Path;
use tdms::data_type::TdmsDataType;
use tdms::TDMSFile;

fn main() {
    // open and parse the TDMS file, passing in metadata false will mean the entire file is
    // read into memory, not just the metadata
    let file = match TDMSFile::from_path(Path::new("data/standard.tdms")) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    // fetch groups
    let groups = file.groups();

    for group in groups {
        // fetch an IndexSet of the group's channels
        let channels = file.channels(&group);

        for (_, channel) in channels {
            // the returned full channel is an iterator over raw data
            // IMPORTANT NOTE: Unless you plan on reading the full channel WITHOUT reading any other channel then
            // you MUST clone the file before pulling multiple channels. If you don't clone the file,
            // the iterators will clash with each other and cause problems. I experimented with having
            // the channels take ownership of file, but the amount of memory lost by having to
            //copy everything over caused problems. Someone better than I needs to fix that.
            let full_channel = match channel.data_type {
                // the returned full channel is an iterator over raw data
                TdmsDataType::DoubleFloat(_) => file.channel_data_double_float(channel),
                _ => {
                    panic!("{}", "channel for data type unimplemented")
                }
            };

            let full_channel_iterator = match full_channel {
                Ok(i) => i,
                Err(e) => {
                    panic!("{:?}", e)
                }
            };

            println!("{:?}", full_channel_iterator.count());
        }
    }
}

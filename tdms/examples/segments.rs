extern crate tdms;

use std::path::Path;
use tdms::TDMSFile;
use tdms_format::data_type::TdmsDataType;

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

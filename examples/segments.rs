extern crate tdms;

use std::path::Path;
use tdms::TDMSFile;

fn main() {
    // open and parse the TDMS file, passing in metadata false will mean the entire file is
    // read into memory, not just the metadata
    let file = match TDMSFile::from_path(Path::new("data/standard.tdms"), false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    // fetch groups
    let groups = file.groups();

    for group in groups {
        // fetch an IndexSet of the group's channels
        let channels = file.channels(&group);

        for channel in channels {
            // once you know the channel's full path (group + channel) you can ask for the full
            // channel object. This contains all the segments that contain this data channel
            // eventually this will also implement Iterator, allowing you to iterate through
            // the channels raw data, currently planned
            match file.channel(&group, &channel) {
                Ok(c) => c,
                Err(e) => panic!("{:?}", e),
            };
        }
    }
}

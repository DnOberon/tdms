extern crate tdms;

use tdms::TDMSFile;

fn main() {
    // open and parse the TDMS file, passing in metadata false will mean the entire file is
    // read into memory, not just the metadata
    let file = match TDMSFile::from_path("data/standard.tdms", false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    // TDMS files record their data in segments - each can potentially contain metadata and/or raw
    // data
    for segment in file.segments {
        println!("{:?}", segment.metadata)
    }
}

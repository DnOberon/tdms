# tdms 

`tdms` is a LabVIEW TDMS file parser library written in Rust. This is meant to be a general purpose library for reading and performing any calculation work on data contained in those files.

**Note:** This library is a work in progress. While I do not expect the current function signatures and library structure to change, you could experience difficulties due to early adoption. 

### Current Features
- Read both standard and big endian encoded files
- Read files with DAQmx data and data indices
- Read all segments in file, along with their groups and channels (per segment only)
- Read all raw data contained in all segments in file (as a `Vec<u8>` only at the present time)


## Usage

```rust
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
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)

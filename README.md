# tdms 

`tdms` is a LabVIEW TDMS file parser library written in Rust. This is meant to be a general purpose library for reading and performing any calculation work on data contained in those files.

**Note:** This library is a work in progress. While I do not expect the current function signatures and library structure to change, you could experience difficulties due to early adoption. Functionality is currently limited. Raw data channel iterators will be added slowly and a list will be kept indicating which are available and which are under construction 

### Current Features
- Read both standard and big endian encoded files
- Read files with DAQmx data and data indices
- Read all segments in file, along with their groups and channels (per segment only)
- Read all raw data contained in all segments in file (as a `Vec<u8>` only at the present time)
- Logging using the `log` api - users of the library must choose and initialize the implementation, such as `env-logger`

Here is a list of all supported iterators for TDMS data types. If completely unlisted, then that type is not supported yet. Check back frequently as this list will grow quickly.

| Data Type                 | Standard           | Interleaved        | DAQmx   |
|---------------------------|--------------------|--------------------|---------|
| Double Float              | &check;            | &check; - untested | &cross; |
| Single Float              | &check; - untested | &check; - untested | &cross; |
| Single Float with unit    | &check; - untested | &check; - untested | &cross; |
| Double Float with unit    | &check; - untested | &check; - untested | &cross; |
| Complex Single Float      | &check; - untested | &check; - untested | &cross; |
| Complex Double Float      | &check; - untested | &check; - untested | &cross; |
| I8                        | &check; - untested | &check; - untested | &cross; |
| I32                       | &check; - untested | &check; - untested | &cross; |
| I64                       | &check; - untested | &check; - untested | &cross; |
| U8                        | &check; - untested | &check; - untested | &cross; |
| U16                       | &check; - untested | &check; - untested | &cross; |
| U32                       | &check; - untested | &check; - untested | &cross; |
| U64                       | &check; - untested | &check; - untested | &cross; |
| Boolean                   | &check; - untested | &check; - untested | &cross; |
| Timestamp (returns tuple) | &check; - untested | &check; - untested | &cross; |
| Single Float              | &check; - untested | &check; - untested | &cross; |
| Single Float              | &check; - untested | &check; - untested | &cross; |
| Single Float              | &check; - untested | &check; - untested | &cross; |



### Planned Features
- Iterators for each channel type, return native Rust values from encoded data channels
- DAQmx data channel iterator support
- Searching on string channels

## Usage

```rust
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

        let mut i = 0;
        for (_, channel) in channels {
            // once you know the channel's full path (group + channel) you can ask for the full
            // channel object. In order to fetch a channel you must call the proper channel func
            // depending on your data type. Currently this feature is unimplemented but the method
            // of calling this is set down for future changes
            let full_channel = match channel.data_type {
                // the returned full channel is an iterator over raw data
                TdmsDataType::DoubleFloat(_) => file.clone().channel_data_double_float(channel),
                _ => {
                    panic!("{}", "channel for data type unimplemented")
                }
            };

            let mut full_channel_iterator = match full_channel {
                Ok(i) => i,
                Err(e) => {
                    panic!("{:?}", e)
                }
            };

            println!("{:?}", full_channel_iterator.count());

            i += 1;
        }
    }
}
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)

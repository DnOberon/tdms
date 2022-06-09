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

    // TDMS files record their data in segments - each can potentially contain metadata and/or raw
    // data
    for segment in file.segments {
        println!("{:?}", segment.metadata)
    }
}

```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)

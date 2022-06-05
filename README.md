# tdms 

`tdms` is a LabVIEW TDMS file parser library written in Rust. This is meant to be a general purpose library for reading and performing any calculation work on data contained in those files.

**Note:** This library is a work in progress. While I do not expect the current function signatures and library structure to change, you could experience difficulties due to early adoption. 

### Current Features
- Read both standard and big endian encoded files
- Read files with DAQmx data and data indices
- Read all segments in file, along with their groups and channels
- Read all raw data contained in all segments in file


## Usage

```rust
let file = match TDMSFile::from_path("data/big_endian.tdms", false) {
    Ok(f) => f,
    Err(e) => panic!("{:?}", e),
};
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)

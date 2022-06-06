use crate::segment::Segment;
use crate::TDMSFile;
use std::fs::File;
use std::path::Path;

#[test]
fn can_read_lead_in() {
    let mut f = File::open(Path::new("data/standard.tdms")).expect("Failure to open file");

    let segment: Segment = match Segment::new(&mut f, false) {
        Ok(s) => s,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(segment.lead_in.tag, [84, 68, 83, 109]);
    assert_eq!(segment.lead_in.version_number, 4713);
    assert_eq!(segment.lead_in.next_segment_offset, 292862);
    assert_eq!(segment.lead_in.raw_data_offset, 4862);
    assert_eq!(segment.start_pos, 0);
    assert_eq!(segment.end_pos, 292890);
}

#[test]
fn can_read_all_segments() {
    let file = match TDMSFile::from_path(Path::new("data/standard.tdms"), false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_segments_data_after() {
    let mut file = match TDMSFile::from_path(Path::new("data/standard.tdms"), true) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    match File::open(Path::new("data/standard.tdms")) {
        Ok(mut r) => match file.segments[0].all_data(&mut r) {
            Ok(data) => match data {
                None => {
                    panic!("unable to retrieve segment data")
                }
                Some(data) => assert_eq!(data.len(), 288000),
            },
            Err(e) => panic!("{:?}", e),
        },
        Err(e) => panic!("{:?}", e),
    }

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_segments_data_after_reader() {
    let mut file = match TDMSFile::from_path(Path::new("data/standard.tdms"), true) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    match File::open(Path::new("data/standard.tdms")) {
        Ok(mut r) => match file.segments[0].all_data_reader(&mut r) {
            Ok(data) => (assert_eq!(data.limit(), 288000)),
            Err(e) => panic!("{:?}", e),
        },
        Err(e) => panic!("{:?}", e),
    }

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_groups_from_segment() {
    let file = match TDMSFile::from_path(Path::new("data/standard.tdms"), false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_segments_be() {
    let file = match TDMSFile::from_path(Path::new("data/big_endian.tdms"), false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_segments_raw() {
    let file = match TDMSFile::from_path(Path::new("data/raw.tdms"), false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(file.segments.len(), 3);
}

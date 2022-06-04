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
    let file = match TDMSFile::from_path("data/standard.tdms", false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_groups_from_segment() {
    let file = match TDMSFile::from_path("data/standard.tdms", false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    if file.segments.len() > 0 {
        let groups = file.segments[0].groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], "EHM");

        let channels = file.segments[0].channels();
        assert_eq!(channels.len(), 18);
        assert_eq!(channels[9], "SensoPowerV");
        //random check of a middle channel to verify we're reading all of them correctly
    } else {
        panic!("no segments to read");
    }

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_segments_be() {
    let file = match TDMSFile::from_path("data/big_endian.tdms", false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    if file.segments.len() > 0 {
        let groups = file.segments[0].groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], "Measured Data");

        let channels = file.segments[0].channels();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[1], "Phase sweep");
        //random check of a middle channel to verify we're reading all of them correctly
    } else {
        panic!("no segments to read");
    }

    assert_eq!(file.segments.len(), 2);
}

#[test]
fn can_read_all_segments_raw() {
    let file = match TDMSFile::from_path("data/raw.tdms", false) {
        Ok(f) => f,
        Err(e) => panic!("{:?}", e),
    };

    if file.segments.len() > 0 {
        let groups = file.segments[0].groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], "Layer Data");

        let channels = file.segments[0].channels();
        assert_eq!(channels.len(), 7);
        assert_eq!(channels[4], "Fifth Chan");
        //random check of a middle channel to verify we're reading all of them correctly
    } else {
        panic!("no segments to read");
    }

    assert_eq!(file.segments.len(), 3);
}

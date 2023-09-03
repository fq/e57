use e57::{CartesianCoordinate, E57Reader, RawValues, RecordName, RecordValue};
use std::fs::File;

#[test]
fn header() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();

    assert_eq!(&header.signature, b"ASTM-E57");
    assert_eq!(header.major, 1);
    assert_eq!(header.minor, 0);
    assert_eq!(header.page_size, 1024);
    assert_eq!(header.phys_length, 743424);
    assert_eq!(header.phys_xml_offset, 740736);
    assert_eq!(header.xml_length, 2172);
}

#[test]
fn validate_crc() {
    let file = File::open("testdata/bunnyDouble.e57").unwrap();
    assert_eq!(E57Reader::validate_crc(file).unwrap(), 1024);

    let file = File::open("testdata/corrupt_crc.e57").unwrap();
    assert!(E57Reader::validate_crc(file).is_err());
}

#[test]
fn raw_xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();

    let reader = File::open("testdata/bunnyDouble.e57").unwrap();
    let xml = E57Reader::raw_xml(reader).unwrap();

    assert_eq!(xml.len(), 2172);
    assert_eq!(xml.len(), header.xml_length as usize);
}

#[test]
fn xml() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let header = reader.header();
    let xml = reader.xml();
    let xml_len = xml.as_bytes().len();

    assert_eq!(xml_len, 2172);
    assert_eq!(xml_len, header.xml_length as usize);
}

#[test]
fn format_name() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let format = reader.format_name();
    assert_eq!(format, "ASTM E57 3D Imaging Data File");
}

#[test]
fn guid() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let guid = reader.guid();
    assert_eq!(guid, "{19AA90ED-145E-4B3B-922C-80BC00648844}");
}

#[test]
fn creation() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let creation = reader.creation().unwrap();
    assert_eq!(creation.gps_time, 987369380.8049808);
    assert_eq!(creation.atomic_reference, false);
}

#[test]
fn empty_extensions() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let extensions = reader.extensions();
    assert_eq!(extensions.len(), 0);
}

#[test]
fn coord_metadata() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let metadata = reader.coordinate_metadata();
    assert_eq!(metadata, Some(""));
}

#[test]
fn pointclouds() {
    let reader = E57Reader::from_file("testdata/bunnyDouble.e57").unwrap();
    let pcs = reader.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert_eq!(pc.guid, "{9CA24C38-C93E-40E8-A366-F49977C7E3EB}");
    assert_eq!(pc.name.as_deref(), Some("bunny"));
    assert_eq!(pc.file_offset, 48);
    assert_eq!(pc.records, 30571);
    assert_eq!(pc.prototype.len(), 4);
    assert!(matches!(pc.prototype[0].name, RecordName::CartesianX,));
    assert!(matches!(pc.prototype[1].name, RecordName::CartesianY,));
    assert!(matches!(pc.prototype[2].name, RecordName::CartesianZ,));
    assert!(matches!(
        pc.prototype[3].name,
        RecordName::CartesianInvalidState,
    ));

    let reader = E57Reader::from_file("testdata/tinyCartesianFloatRgb.e57").unwrap();
    let pcs = reader.pointclouds();
    assert_eq!(pcs.len(), 1);
    let pc = pcs.first().unwrap();
    assert_eq!(pc.guid, "{49aa8f8b-618f-423e-a632-f9a58ad79e40}");
    assert_eq!(pc.name.as_deref(), Some("exp2.fls.subsampled"));
    assert_eq!(pc.file_offset, 48);
    assert_eq!(pc.records, 2090);
    assert_eq!(pc.prototype.len(), 6);
    assert!(matches!(pc.prototype[0].name, RecordName::CartesianX,));
    assert!(matches!(pc.prototype[1].name, RecordName::CartesianY,));
    assert!(matches!(pc.prototype[2].name, RecordName::CartesianZ,));
    assert!(matches!(pc.prototype[3].name, RecordName::ColorRed,));
    assert!(matches!(pc.prototype[4].name, RecordName::ColorGreen,));
    assert!(matches!(pc.prototype[5].name, RecordName::ColorBlue,));
}

#[test]
fn bunny_point_count() {
    let files = [
        "testdata/bunnyDouble.e57",
        "testdata/bunnyFloat.e57",
        "testdata/bunnyInt32.e57",
        "testdata/bunnyInt24.e57",
        "testdata/bunnyInt21.e57",
        "testdata/bunnyInt19.e57",
    ];
    for file in files {
        let mut reader = E57Reader::from_file(file).unwrap();
        let pcs = reader.pointclouds();
        let pc = pcs.first().unwrap();
        assert_eq!(pc.records, 30571);
        let points: Vec<RawValues> = reader
            .pointcloud_raw(pc)
            .unwrap()
            .map(|p| p.unwrap())
            .collect();
        assert_eq!(points.len(), 30571);
    }
}

#[test]
fn cartesian_bounds() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let bounds = pc.cartesian_bounds.as_ref().unwrap();
    assert_eq!(bounds.x_min, Some(-9.779529571533203));
    assert_eq!(bounds.x_max, Some(-6.774238109588623));
    assert_eq!(bounds.y_min, Some(4.5138792991638184));
    assert_eq!(bounds.y_max, Some(7.5154604911804199));
    assert_eq!(bounds.z_min, Some(295.52468872070312));
    assert_eq!(bounds.z_max, Some(298.53216552734375));
}

#[test]
fn color_limits() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    let limits = pc.color_limits.as_ref().unwrap();
    assert_eq!(limits.red_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.red_max, Some(RecordValue::Integer(255)));
    assert_eq!(limits.green_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.green_max, Some(RecordValue::Integer(255)));
    assert_eq!(limits.blue_min, Some(RecordValue::Integer(0)));
    assert_eq!(limits.blue_max, Some(RecordValue::Integer(255)));
}

#[test]
fn raw_iterator() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    assert_eq!(pc.records, 2090);
    let mut counter = 0;
    for p in reader.pointcloud_raw(pc).unwrap() {
        let p = p.unwrap();
        assert_eq!(p.len(), 6);
        assert!(matches!(p[0], RecordValue::Single(..)));
        assert!(matches!(p[1], RecordValue::Single(..)));
        assert!(matches!(p[2], RecordValue::Single(..)));
        assert!(matches!(p[3], RecordValue::Integer(..)));
        assert!(matches!(p[4], RecordValue::Integer(..)));
        assert!(matches!(p[5], RecordValue::Integer(..)));
        counter += 1;
    }
    assert_eq!(counter, pc.records);
}

#[test]
fn simple_iterator() {
    let file = "testdata/tinyCartesianFloatRgb.e57";
    let mut reader = E57Reader::from_file(file).unwrap();
    let pcs = reader.pointclouds();
    let pc = pcs.first().unwrap();
    assert_eq!(pc.records, 2090);
    let mut counter = 0;
    for p in reader.pointcloud_simple(pc).unwrap() {
        let p = p.unwrap();
        assert!(matches!(p.cartesian, CartesianCoordinate::Valid { .. }));
        assert!(matches!(p.color, Some(..)));
        counter += 1;
    }
    assert_eq!(counter, pc.records);
}

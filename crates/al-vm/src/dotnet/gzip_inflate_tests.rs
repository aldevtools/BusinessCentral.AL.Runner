use super::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;

fn gzip(plain: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(plain).unwrap();
    enc.finish().unwrap()
}

fn raises(r: Result<Vec<u8>, Signal>) -> bool {
    matches!(r, Err(Signal::Error(_)))
}

fn refuses(r: Result<Vec<u8>, Signal>) -> bool {
    matches!(r, Err(Signal::Unsupported(_)))
}

fn text(n: usize) -> Vec<u8> {
    b"corrupt me after the header "
        .iter()
        .copied()
        .cycle()
        .take(n)
        .collect()
}

fn flip(mut bytes: Vec<u8>, at: usize) -> Vec<u8> {
    bytes[at] = 255 - bytes[at];
    bytes
}

#[test]
fn short_streams_other_than_the_measured_ones_refuse() {
    assert!(inflate(&[31]).unwrap().is_empty());
    assert!(raises(inflate(&[1, 2, 3, 4, 5])));
    assert!(refuses(inflate(&[0])));
    assert!(refuses(inflate(&[31, 139, 8])));
    assert!(refuses(inflate(&[1, 2, 3, 4, 6])));
}

#[test]
fn an_unknown_method_raises_only_in_the_measured_shape() {
    let mut header = vec![31, 139, 99, 0, 0, 0, 0, 0, 0, 3, 1, 2];
    assert!(raises(inflate(&header)));
    header[2] = 7;
    assert!(refuses(inflate(&header)), "another method");
    header[2] = 99;
    header[3] = 0x10;
    header.push(0);
    assert!(refuses(inflate(&header)), "a header with a comment field");
    // Only the measured 12-byte stream raises; other method-99 streams keep the refusal.
    let measured = [31, 139, 99, 0, 0, 0, 0, 0, 0, 3, 1, 2];
    assert!(refuses(inflate(&measured[..10])), "the header alone");
    let then_member = [&measured[..10], &gzip(b"A")[..]].concat();
    assert!(refuses(inflate(&then_member)), "the header then a valid member");
    let mut other_header = measured;
    other_header[4] = 7;
    other_header[9] = 0;
    assert!(refuses(inflate(&other_header)), "other time and OS bytes");
}

#[test]
fn a_corrupt_first_block_raises_only_in_the_measured_shapes() {
    let header = [31, 139, 8, 0, 0, 0, 0, 0, 0, 3];
    let with = |data: &[u8]| [&header[..], data].concat();
    assert!(raises(inflate(&with(&[255; 20]))));
    assert!(raises(inflate(&with(&[65; 20]))));
    // Main's refusal stays for the shapes the corpus did not measure: a reserved block that is not the last, a stored
    // block whose length fields differ.
    assert!(refuses(inflate(&with(&[0xfe; 20]))));
    assert!(refuses(inflate(&with(&[1, 0x41, 0x41, 0x42, 0x42, 0, 0]))));
    // Only the two measured byte runs raise; other blocks of the same type keep the refusal.
    for data in [
        &[255][..],
        &[7],
        &[0xe7; 20],
        &[7, b'c', b'o', b'r', b'r', b'u', b'p', b't'],
        &[65; 5],
        &[1, 0, 0, 0, 0],
        &[1, 5, 0, 5, 0, 1, 2, 3, 4, 5],
        &[0xc1, 3, 0, 3, 0, 1, 2, 3],
    ] {
        assert!(refuses(inflate(&with(data))), "first block {data:?}");
    }
}

#[test]
fn a_trailer_mismatch_raises_only_for_the_measured_member() {
    let packed = gzip(&text(CORRUPT_MEMBER_TEXT_LEN));
    let last = packed.len() - 1;
    assert!(raises(inflate(&flip(packed.clone(), last))), "size field");
    assert!(raises(inflate(&flip(packed.clone(), last - 4))), "CRC32 field");
    let longer = gzip(&text(CORRUPT_MEMBER_TEXT_LEN + 1));
    assert!(
        refuses(inflate(&flip(longer.clone(), longer.len() - 1))),
        "a longer member"
    );
    for n in [0, 1, 2, 40, CORRUPT_MEMBER_TEXT_LEN - 1] {
        let shorter = gzip(&text(n));
        let end = shorter.len();
        assert!(
            refuses(inflate(&flip(shorter.clone(), end - 1))),
            "size field of a {n}-byte member"
        );
        assert!(
            refuses(inflate(&flip(shorter.clone(), end - 5))),
            "CRC32 field of a {n}-byte member"
        );
    }
    let joined = [gzip(b"first"), flip(packed.clone(), last)].concat();
    assert!(refuses(inflate(&joined)), "the second member");
    let followed = [flip(packed.clone(), last), vec![1, 2, 3]].concat();
    assert!(refuses(inflate(&followed)), "bytes after the member");
}

#[test]
fn a_member_with_a_name_field_still_decodes() {
    let mut enc = flate2::GzBuilder::new()
        .filename("a.txt")
        .write(Vec::new(), Compression::default());
    enc.write_all(b"named").unwrap();
    let packed = enc.finish().unwrap();
    assert_eq!(inflate(&packed).unwrap(), b"named");
    assert!(
        refuses(inflate(&flip(packed.clone(), packed.len() - 1))),
        "its trailer mismatch refuses"
    );
}

#[test]
fn a_trailer_cut_after_its_crc32_keeps_the_decoded_bytes_when_the_crc32_matches() {
    let packed = gzip(b"cut in the size field");
    let cut = &packed[..packed.len() - 4];
    assert_eq!(inflate(cut).unwrap(), b"cut in the size field");
    let shorter = &packed[..packed.len() - 6];
    assert_eq!(
        inflate(shorter).unwrap(),
        b"cut in the size field",
        "two CRC32 bytes are not checked"
    );
    assert!(
        refuses(inflate(&flip(cut.to_vec(), cut.len() - 1))),
        "a CRC32 that differs"
    );
}

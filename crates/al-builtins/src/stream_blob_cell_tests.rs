//! Tests of `stream::BlobCell`: readers share it, a writer excludes them.
use super::*;

#[test]
fn readers_share_and_a_writer_excludes_them_as_in_a_refcell() {
    let b = new_blob();
    let (r1, r2) = (b.borrow(), b.borrow());
    assert_eq!(r1.bytes, r2.bytes);
    drop((r1, r2));
    b.borrow_mut().bytes.push(1);
    assert_eq!(b.borrow().bytes, vec![1]);
}

#[test]
#[should_panic(expected = "already borrowed")]
fn a_writer_while_a_reader_is_held_panics() {
    let b = new_blob();
    let _r = b.borrow();
    let _w = b.borrow_mut();
}

#[test]
fn a_blob_is_read_by_two_threads() {
    let b = new_blob();
    b.borrow_mut().bytes.extend_from_slice(b"abc");
    let copies: Vec<Vec<u8>> = std::thread::scope(|scope| {
        let hs: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| copy_blob(&b, false).borrow().bytes.clone()))
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(copies, vec![b"abc".to_vec(), b"abc".to_vec()]);
    assert!(Value::Blob(b).is_plain());
}

// streams/TestStreamsRuntimeContracts.al:WriteText_ExplicitEncodings_Bytes, streams/StTEncodings.al:RoundTripSameEncoding and
/// π is written as `p` on an MS-DOS and a Windows stream (streams/StTEncodings.al:UnmappableCharacters); a box-drawing
/// character of code page 437 is not pinned.
#[test]
fn pi_is_written_as_p() {
    assert_eq!(Encoding::MsDos.encode("aπb"), b"apb".to_vec());
    assert_eq!(Encoding::Windows.encode("aπb"), b"apb".to_vec());
    assert!(Encoding::MsDos.encode_pinned("π") && Encoding::Windows.encode_pinned("π"));
    assert!(!Encoding::MsDos.encode_pinned("\u{2500}"));
}

/// The bytes pinned earlier one by one (`CP437_AS_READ`) read as the full table says.
#[test]
fn the_earlier_pins_agree_with_the_msdos_table() {
    for (b, c) in CP437_AS_READ {
        assert_eq!(Encoding::MsDos.decode(&[b]), c.to_string(), "{b:#x}");
    }
}

/// `§` as 0x15 is measured for `OutStream.WriteText` only (`out_write_text`); the shared encoder that File.Write, the
/// XMLport writer and the Char-to-Byte conversion use keeps main's byte.
#[test]
fn the_section_sign_rule_stays_out_of_the_shared_encoder() {
    assert_eq!(Encoding::MsDos.encode("a\u{A7}b"), b"a?b".to_vec());
    assert!(!Encoding::MsDos.encode_pinned("\u{A7}"));
}

// CrossEncodingUtf8ToOthers: the default encoding is code page 437.
#[test]
fn the_default_encoding_is_code_page_437() {
    assert_eq!(Encoding::default(), Encoding::MsDos);
    assert_eq!(Encoding::MsDos.encode("hé€Ж"), vec![0x68, 0x82, 0xB1, 0x3F]);
    assert_eq!(Encoding::Windows.encode("hé"), vec![0x68, 0xE9]);
    // The euro sign comes back from its byte; the bytes whose character the Windows tier changes read as pinned.
    assert_eq!(Encoding::MsDos.decode(&[0xB1]), "€");
    assert_eq!(
        Encoding::MsDos.decode(&[0x61, 0xC3, 0xA4, 0xE2, 0x82]),
        "a\u{201C}ñË\u{e9}"
    );
    assert!(Encoding::MsDos.decode_pinned(&[0x82, 0xA4, 0xC3]));
    // A box-drawing byte nothing pins is not decoded.
    assert!(!Encoding::MsDos.decode_pinned(&[0xC4]));
    // The UTF-8 bytes of U+202F and of `年月日` in a SaveAs Xml dataset read through the default encoding
    // (reports/runtime/YReportPropsTests.al:80, :96).
    let ja = "2025\u{5E74}1\u{6708}31\u{65E5} 1\u{202F}2".as_bytes();
    assert!(Encoding::MsDos.decode_pinned(ja));
    assert_eq!(
        Encoding::MsDos.decode(ja),
        "2025\u{CE}\u{2030}\u{201E}1\u{B5}\u{A3}\u{EA}31\u{B5}\u{F9}\u{D1} 1\u{CB}\u{C7}\u{BB}2"
    );
}

// al-runner issue 6853: a call stack text written to a default (code page 437) stream reads back with its mark, as it does
// from a Windows stream (error-handling/lt-wd/WdCallStackToBlob.al:96-102).
#[test]
fn the_call_stack_mark_round_trips_through_the_default_encoding() {
    let t = "\"C\"(CodeUnit 50100).Collect line 3\r\n\u{FDD0}";
    let bytes = Encoding::MsDos.encode(t);
    assert!(Encoding::MsDos.decode_pinned(&bytes));
    assert_eq!(Encoding::MsDos.decode(&bytes), t);
}

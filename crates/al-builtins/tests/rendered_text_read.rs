//! The refusals of the one place a marked text starts (`rendered_text::read_whole`, ADR-040 amendment 2026-11-12, A2): a
//! buffer a marked text was written to reads back whole only on a UTF-8 stream, without a byte order mark, with UTF-8 bytes
//! and every mark whole.

use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, Form, Source};

fn read(bytes: &[u8], utf8: bool) -> Result<(String, usize), Signal> {
    rendered_text::read_whole(Source::Written { bytes }, true, false, utf8, "InStream.Read(Text)")
}

fn refused(r: Result<(String, usize), Signal>, want: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{why}"),
        other => panic!("expected a refusal naming {want}, got {other:?}"),
    }
}

fn written() -> String {
    let n = rendered_text::register("Read Report", "Html", "WordOne").expect("a render");
    format!("Sig {}", rendered_text::mark_text(n, Form::Whole))
}

#[test]
fn a_written_buffer_reads_back_whole() {
    let text = written();
    assert_eq!(read(text.as_bytes(), true).unwrap().0, text);
}

#[test]
fn a_written_buffer_that_starts_with_a_byte_order_mark_refuses() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(written().as_bytes());
    refused(read(&bytes, true), "starts with a byte order mark");
}

#[test]
fn a_written_buffer_whose_bytes_are_not_utf8_refuses() {
    let mut bytes = written().into_bytes();
    bytes.push(0xFF);
    refused(read(&bytes, true), "whose bytes are not UTF-8");
}

#[test]
fn a_written_buffer_on_a_stream_that_is_not_utf8_refuses() {
    refused(read(written().as_bytes(), false), "on a stream that is not UTF-8");
}

#[test]
fn a_mark_that_is_not_whole_refuses() {
    // The mark without its number character, and a mark whose number names no render.
    let lone = format!("Sig {}", rendered_text::RENDERED_TEXT_MARK);
    refused(read(lone.as_bytes(), true), "a mark that is not whole");
    let unknown = format!(
        "{}{}",
        rendered_text::RENDERED_TEXT_MARK,
        char::from_u32(0xF0000 + 0xFFF0).unwrap()
    );
    refused(read(unknown.as_bytes(), true), "a mark that is not whole");
}

#[test]
fn an_html_placeholder_without_a_render_number_refuses() {
    let html = al_builtins::placeholder_output::HTML;
    let r = rendered_text::read_whole(
        Source::Html {
            render: None,
            bytes: html,
        },
        true,
        false,
        true,
        "InStream.Read(Text)",
    );
    refused(r, "has no render number");
}

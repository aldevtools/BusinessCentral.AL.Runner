//! The render numbers of a process run out (ADR-040 amendment 2026-11-12, A1) once it has as many distinct renders as a number
//! character can name: `register` then gives no number, and the read of the Html placeholder that has none refuses
//! (`rendered_text_read.rs`). A test binary of its own: the numbers are process-wide, so using them up would starve the tests
//! beside it.

use al_builtins::rendered_text;

#[test]
fn register_gives_no_number_once_the_numbers_are_used_up() {
    let mut given = 0u32;
    while rendered_text::register(&format!("Numbers Report {given}"), "Html", "WordOne").is_some() {
        given += 1;
        assert!(given <= 0xFFFE, "more numbers than a number character can name");
    }
    assert_eq!(given, 0xFFFE);
    assert!(rendered_text::register("Numbers Report", "Html", "WordTwo").is_none());
    // A render the process has already gets its number.
    assert_eq!(rendered_text::register("Numbers Report 0", "Html", "WordOne"), Some(0));
}

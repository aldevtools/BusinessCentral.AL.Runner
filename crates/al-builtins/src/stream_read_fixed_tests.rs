//! `read_fixed` (one borrow of the buffer) against the end check and the read of `in_read`'s general path (two `with_in`
//! calls): the same answer, position, end flag and use counts, on every kind of stream it answers.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

/// What a read left behind: its answer (bytes, the first read at the end, or the error's text) and the stream's state.
#[derive(Debug, PartialEq)]
struct After {
    answer: Result<Option<Vec<u8>>, String>,
    pos: usize,
    hit_end: bool,
    uses: u32,
    in_uses: Option<u32>,
}

fn stream(s: InState) -> Value {
    Value::Library(
        crate::library_kind::LibraryKind::InStream,
        Rc::new(RefCell::new(LibraryValue::InStream(s))),
    )
}

fn shown(e: Signal) -> String {
    match e {
        Signal::Error(e) => format!("{}|{}", e.message, e.code),
        Signal::Unsupported(m) => format!("unsupported: {m}"),
        _ => "other".into(),
    }
}

/// The end check and the read of the general path, as `in_read` makes them.
fn general(v: &mut Value, width: usize) -> Result<Option<Vec<u8>>, Signal> {
    let first_at_end = with_in(Some(v), |s| {
        let at = s.pos >= s.len();
        let first = at && !s.hit_end;
        if at {
            s.hit_end = true;
        }
        Ok(first)
    })?;
    if first_at_end {
        return Ok(None);
    }
    with_in(Some(v), |s| {
        let pos = s.pos;
        let got = s
            .with_data(|d| d.get(pos..pos + width).map(<[u8]>::to_vec))
            .ok_or_else(crate::stream_read_len::invalid_data)?;
        s.pos += width;
        Ok(Some(got))
    })
}

fn after(v: &Value, answer: Result<Option<Vec<u8>>, Signal>) -> After {
    let Value::Library(_, h) = v else { unreachable!() };
    let LibraryValue::InStream(s) = &*h.borrow() else {
        unreachable!()
    };
    After {
        answer: answer.map_err(shown),
        pos: s.pos,
        hit_end: s.hit_end,
        uses: s.uses,
        in_uses: s.live_blob().map(|b| b.borrow().line_breaks.in_uses),
    }
}

/// A named way to make a stream.
type Case = (&'static str, fn() -> InState);

/// The streams `read_fixed` answers: a snapshot, a loaded and an unloaded buffer, empty or not, at the end, past it, and one
/// a use refuses.
fn cases() -> Vec<Case> {
    fn live(bytes: &[u8], loaded: bool) -> InState {
        let b = new_blob();
        b.borrow_mut().bytes.extend_from_slice(bytes);
        b.borrow_mut().loaded = loaded;
        InState::live(b, Encoding::default())
    }
    vec![
        ("snapshot of 5", || {
            InState::snapshot(vec![1, 2, 3, 4, 5], Encoding::default())
        }),
        ("empty snapshot", || InState::snapshot(Vec::new(), Encoding::default())),
        ("live of 5", || live(&[9, 8, 7, 6, 5], true)),
        ("empty live", || live(&[], true)),
        ("unloaded live", || live(&[1, 2, 3], false)),
        ("live after an end hit", || {
            let mut s = live(&[1], true);
            s.pos = 1;
            s.hit_end = true;
            s
        }),
        ("live handed to content", || {
            let mut s = live(&[1, 2], true);
            s.handed_to_content = true;
            s
        }),
        ("snapshot not initialized", || {
            let mut s = InState::snapshot(vec![1], Encoding::default());
            s.not_initialized = true;
            s
        }),
        ("live after a line break call", || {
            let mut s = live(&[1, 2], true);
            s.after_line_break_call = true;
            s
        }),
    ]
}

#[test]
fn read_fixed_does_what_the_end_check_and_the_read_do() {
    for (name, make) in cases() {
        for width in [1, 4, 8] {
            let (mut a, mut b) = (stream(make()), stream(make()));
            // Read until both raised, so every position, the end and past it are compared.
            for step in 0..8 {
                let want = general(&mut a, width);
                let raised = want.is_err();
                let got = read_fixed(Some(&mut b), width)
                    .expect("read_fixed answers this stream")
                    .map(|r| r.map(|bytes| bytes[..width].to_vec()));
                assert_eq!(after(&b, got), after(&a, want), "{name}, width {width}, read {step}");
                if raised {
                    break;
                }
            }
        }
    }
}

#[test]
fn read_fixed_leaves_a_pending_buffer_and_other_values_to_the_general_path() {
    let b = new_blob();
    b.borrow_mut().render_pending = Some("a report".into());
    let mut pending = stream(InState::live(b.clone(), Encoding::default()));
    assert!(read_fixed(Some(&mut pending), 1).is_none());
    // Nothing was counted.
    assert_eq!(after(&pending, Ok(None)).uses, 0);
    assert_eq!(b.borrow().line_breaks.in_uses, 0);
    let mut uncreated = Value::Library(
        crate::library_kind::LibraryKind::InStream,
        Rc::new(RefCell::new(LibraryValue::Empty)),
    );
    assert!(read_fixed(Some(&mut uncreated), 1).is_none());
    assert!(read_fixed(Some(&mut Value::Integer(1)), 1).is_none());
    assert!(read_fixed(None, 1).is_none());
}

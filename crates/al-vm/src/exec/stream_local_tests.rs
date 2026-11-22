//! Which builtin sites run on the stream path (`stream_local`); every other site keeps `run_builtin`.

use super::stream_local;
use al_builtins::builtin_id::ids;
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::RtType;
use al_ir as ir;

const IN: RtType = RtType::Library(LibraryKind::InStream);
const OUT: RtType = RtType::Library(LibraryKind::OutStream);

fn local(i: usize) -> ir::Arg {
    ir::Arg::Ref(ir::Place::Local(i))
}

fn site(
    id: al_builtins::builtin_id::BuiltinId,
    reg: Option<u16>,
    recv: Option<&ir::Arg>,
    n: usize,
    ty: &RtType,
) -> Option<u16> {
    stream_local(id, reg, recv, n, |_| Some(ty))
}

#[test]
fn a_read_or_write_on_a_stream_local_in_place_takes_the_stream_path() {
    assert_eq!(site(ids::INSTREAM_READ_1, Some(3), Some(&local(3)), 1, &IN), Some(3));
    assert_eq!(site(ids::INSTREAM_READ_9, Some(3), Some(&local(3)), 2, &IN), Some(3));
    assert_eq!(site(ids::OUTSTREAM_WRITE_1, Some(2), Some(&local(2)), 1, &OUT), Some(2));
}

#[test]
fn every_other_site_keeps_run_builtin() {
    // Another stream method.
    assert_eq!(site(ids::INSTREAM_READTEXT_1, Some(3), Some(&local(3)), 1, &IN), None);
    assert_eq!(site(ids::INSTREAM_EOS_1, Some(3), Some(&local(3)), 0, &IN), None);
    // A global, a field or no receiver.
    assert_eq!(
        site(
            ids::INSTREAM_READ_1,
            Some(9),
            Some(&ir::Arg::Ref(ir::Place::Global(0))),
            1,
            &IN
        ),
        None
    );
    assert_eq!(site(ids::INSTREAM_READ_1, None, None, 1, &IN), None);
    // The receiver read into a temporary first (`arg_reg`).
    assert_eq!(site(ids::INSTREAM_READ_1, Some(7), Some(&local(3)), 1, &IN), None);
    // A local of another declared type (a Variant that holds a stream).
    assert_eq!(
        site(ids::INSTREAM_READ_1, Some(3), Some(&local(3)), 1, &RtType::Variant),
        None
    );
    // More than two arguments.
    assert_eq!(site(ids::OUTSTREAM_WRITE_1, Some(2), Some(&local(2)), 3, &OUT), None);
}

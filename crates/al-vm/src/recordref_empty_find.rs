//! What `GetLastErrorText(true)` answers after a RecordRef find on an empty set. The error, its code and its unflagged text
//! are the ones a Record's find raises (records/op_inner.rs); only the flagged text has a rule of its own here, because the
//! corpus measured it for a RecordRef apart from a Record (errors/last-error/LastErrorFlagRuntimeCodes.al RecordRef_*_ExcludeFlag,
//! the same on BC 27.5, 28.5 and 29.0; corpus pull request 1327, al-runner issue 7806).

use crate::Session;
use crate::records::Rec;
use al_builtins::native::{ErrorOrigin, Signal};
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

impl Session {
    /// A RecordRef method that forwards to the record method of the same name (`Find`, `FindFirst`, `Insert`, ...).
    pub(crate) fn recordref_record_op(&mut self, method: &str, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let op = RecordOp::from_name(method).expect("a RecordRef method that forwards to the record");
        let which = match args.first() {
            Some(v) => al_builtins::native::text_of(v).map(|t| t.to_string()),
            None => Some("=".to_string()),
        };
        let temporary = rec.borrow().temporary;
        let r = self.record_op_of_al(op, rec, args);
        match r {
            Err(Signal::Error(mut e))
                if matches!(
                    op,
                    RecordOp::Find | RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet
                ) =>
            {
                if let Some(origin) = empty_find_origin(op, which.as_deref(), temporary, &e.code, &e.message, &e.origin)
                {
                    e.origin = origin;
                }
                Err(Signal::Error(e))
            }
            r => r,
        }
    }
}

/// The flagged text of a RecordRef find on an empty set, or `None` for an error that is not one.
/// - The table is empty (DB:TableIsEmpty): the text as it is, for FindFirst, FindLast, FindSet, Find('-') and Find('+') of a
///   database table (RecordRef_EmptyTable_NoFilter_ExcludeFlag) and for Find('-') and Find('+') of a temporary one
///   (RecordRef_Temporary_ExcludeFlag). A temporary FindFirst, FindLast or FindSet: not measured.
/// - Nothing within the filter (DB:NothingInsideFilter): the filters hidden, for the five finds of a database table, empty or
///   with rows (RecordRef_EmptyTable_Filtered_ExcludeFlag, RecordRef_TableWithRows_FilteredOut_ExcludeFlag), and for FindFirst,
///   FindLast and FindSet of a temporary one (RecordRef_Temporary_ExcludeFlag). A temporary Find('-') or Find('+'): not measured.
/// - Find('=') of an empty table (DB:RecordNotFound): the key hidden (RecordRef_EmptyTable_NoFilter_ExcludeFlag `equal_true`). Only
///   the shape the Record rule already gives that text for keeps it (an empty database table with the blank key); every other
///   shape is not measured.
fn empty_find_origin(
    op: RecordOp,
    which: Option<&str>,
    temporary: bool,
    code: &str,
    message: &str,
    record_origin: &ErrorOrigin,
) -> Option<ErrorOrigin> {
    let minus_plus = op == RecordOp::Find && matches!(which, Some("-" | "+"));
    let first_last_set = matches!(op, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet);
    match code {
        "DB:TableIsEmpty" => Some(if !temporary || minus_plus {
            ErrorOrigin::SameText
        } else {
            ErrorOrigin::Unknown
        }),
        "DB:NothingInsideFilter" => Some(match message.find("Filters: ") {
            Some(at) if (first_last_set || (minus_plus && !temporary)) => {
                ErrorOrigin::Redacted(format!("{}<redacted>", &message[..at + "Filters: ".len()]))
            }
            _ => ErrorOrigin::Unknown,
        }),
        "DB:RecordNotFound" => Some(match record_origin {
            ErrorOrigin::Redacted(_) if op == RecordOp::Find && which == Some("=") && !temporary => {
                record_origin.clone()
            }
            _ => ErrorOrigin::Unknown,
        }),
        _ => None,
    }
}

//! The .NET frames of `Exception.ToString()` for the shapes the corpus pins line by line
//! (tests/al-language-onprem-dotnet/lt-y89-tostring-frames/Y89ToStringFrames.al, BC 27, 28, 29): an error an asserterror caught,
//! and an error a TryFunction caught, when the error is raised in procedures of one other codeunit that the caller (or the
//! TryFunction) calls. Any other trace is written by `last_error::exception_text` under `exception-tostring-stack-frames`.

use super::last_error::{codeunit_frame, is_try_function, plain_procedure};
use crate::Session;
use al_builtins::dotnet_sigs::{AL_EXCEPTION, DIVIDE_BY_ZERO_EXCEPTION, NCL_DIALOG_EXCEPTION};

const ASSERT: &str = "\n--- End of stack trace from previous location ---\n   at Microsoft.Dynamics.Nav.Runtime.NavMethodScope.AssertErrorAsync(Func`1 body)";
const TRY_INVOKE: &str =
    "\n   at Microsoft.Dynamics.Nav.Runtime.NavApplicationObjectBase.TryInvokeAsync(NavSession session, Func`1 method)";
const VALUE_TASK: &str =
    "\n   at Microsoft.Dynamics.Nav.Types.ValueTaskExtensions.ManualResetValueTaskSource`1.GetResult(Int16 token)";

fn al_frame(id: u32, name: &str) -> String {
    format!("\n   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit{id}.{name}()")
}

/// The text of the exception when its call stack `frames` (innermost first, the last frame the procedure holding the
/// asserterror or the call of the TryFunction) has a pinned shape, `None` otherwise.
///
/// Pinned shapes (Y89ToStringFrames.al, the corpus lines are the five `Y89_*` tests):
/// - Y89_Asserterror_Plain: an Error() in the procedure that holds the asserterror: the head, the end-of-trace line, the
///   AssertErrorAsync frame;
/// - Y89_Asserterror_Deep, Y89_Try_Deep: a dialog error raised in one or more procedures of another codeunit: their frames, from
///   BC 29 a ValueTaskExtensions frame, then the end-of-trace line and AssertErrorAsync, or the TryFunction's frame and TryInvokeAsync;
/// - Y89_Asserterror_Divide, Y89_Try_Divide: a division by zero in one procedure of another codeunit: the inner exception line,
///   the frame, the end of the inner trace, the frame again, then as above.
///
/// The corpus also counts one compiler-generated line (`b__`) and does not show it: it is not written (the five Y89 cases still
/// fail on that count until a probe shows the line).
pub(super) fn pinned_trace(session: &Session, type_name: &str, msg: &str, frames: &[&str]) -> Option<String> {
    if type_name != NCL_DIALOG_EXCEPTION && type_name != AL_EXCEPTION {
        return None;
    }
    let parsed: Vec<(u32, &str)> = frames.iter().map(|f| codeunit_frame(f)).collect::<Option<_>>()?;
    let &(caller_id, caller) = parsed.last()?;
    let mut out = format!("{type_name}: {msg}");
    // Error() in the procedure that holds the asserterror itself (Y89_Asserterror_Plain) is not written here: Y76DotNet.al:151-152
    // Y76_Asserterror_Error reads the head up to the first `   at `, so a frame line stands right after the head, and that
    // generated line (`b__`) is not shown by the corpus. The generic trace keeps that baseline case passing.
    if parsed.len() == 1 {
        return None;
    }
    let (inner_id, first) = parsed[0];
    let run = parsed.iter().take_while(|(id, _)| *id == inner_id).count();
    if run == parsed.len() || first == "OnRun" {
        return None;
    }
    let callees = &parsed[..run];
    if callees.iter().any(|(id, name)| {
        *name == "OnRun" || !plain_procedure(session, *id, name) || is_try_function(session, *id, name)
    }) {
        return None;
    }
    if type_name == AL_EXCEPTION && callees.len() != 1 {
        return None;
    }
    // Asserterror: the caller is the last frame. TryFunction: the TryFunction's frame, then the caller.
    let rest = &parsed[run..];
    let try_frame = match rest {
        [_] if !is_try_function(session, caller_id, caller) => None,
        [(id, name), _] if is_try_function(session, *id, name) => Some((*id, *name)),
        _ => return None,
    };
    if inner_id == caller_id {
        return None;
    }
    if type_name == AL_EXCEPTION {
        out.push_str(&format!("\n ---> {DIVIDE_BY_ZERO_EXCEPTION}: {msg}"));
    }
    for (id, name) in callees {
        out.push_str(&al_frame(*id, name));
    }
    if type_name == AL_EXCEPTION {
        out.push_str("\n   --- End of inner exception stack trace ---");
        for (id, name) in callees {
            out.push_str(&al_frame(*id, name));
        }
    }
    if session.has(al_version::Gate::ExceptionTraceValueTaskFrameFrom29) {
        out.push_str(VALUE_TASK);
    }
    match try_frame {
        Some((id, name)) => {
            out.push_str(&al_frame(id, name));
            out.push_str(TRY_INVOKE);
        }
        None => {
            out.push_str(ASSERT);
        }
    }
    Some(out)
}

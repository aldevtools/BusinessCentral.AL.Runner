//! The AL call stack of an error (`GetLastErrorCallStack`).
//!
//! The VM keeps the routine and the current statement position of every frame on the stack
//! (`Session::call_stack`, set by `Stmt::Pos` markers). An error is not given its stack when it
//! is raised: each frame it unwinds through adds its own line, innermost first, and the frame
//! that catches it (`asserterror`, a TryFunction call, a guarded `Codeunit.Run`) adds the last.
//!
//! The corpus pins the text (codeunit/TestCodeunitAlCallStack.al, and on BC 27.5
//! session/TestSessionInformationCallstackContracts.al for codeunit, table, field-trigger and event
//! frames and for the line counting):
//!
//! ```text
//! "Object Name"(CodeUnit 60212).RaiseError line 2 - App Name by Publisher version 1.0.0.0
//! ```
//!
//! One frame per line, innermost first (`SessionInformation.Callstack` and `GetLastErrorCallStack`
//! show a backslash after each frame instead of the line feed kept here; corpus
//! session/TestSessionInformationCallstackContracts.al:12-14); a trigger is `OnRun(Trigger)`; `line` is the line of the
//! statement that was running minus the line of the `procedure` or `trigger` keyword. A frame the
//! format is not known for (an object type that has not been measured, a routine
//! with no source) puts `UNTRACKED_CALL_STACK` in the stack, and no text function reads a text
//! that holds it.

use crate::{CallFrame, Session};
use al_builtins::native::{AlError, UNTRACKED_CALL_STACK};
use al_ir as ir;

impl Session {
    /// Adds the frame at the top of the call stack to the error's stack. No frame, no change.
    pub(crate) fn add_top_frame(&self, e: &mut AlError) {
        if let (Some(&object), Some(&frame)) = (self.frames.last(), self.call_stack.last()) {
            self.add_frame(e, object, frame);
        }
    }

    /// Moves the current statement position of the top frame to `p` for the error being raised, without a coverage hit (the
    /// header of a DotNet `foreach`: `Compiler::foreach`).
    pub(crate) fn at_line_of(&mut self, p: u32) {
        if let Some(top) = self.call_stack.last_mut() {
            top.pos = p;
        }
    }

    /// Sets the frames of the test runner that stand below a test's frames (the corpus job and
    /// `al-runner` set them; a session without a runner has none).
    pub fn set_runner_frames(&mut self, frames: String) {
        self.runner_frames = frames;
    }

    /// `SessionInformation.Callstack`: the live frames, innermost first, a backslash after each,
    /// then the runner's. `None` where a frame has no pinned format (the caller refuses).
    pub(crate) fn live_call_stack(&self) -> Option<String> {
        if self.events.unpinned_event_depth > 0 {
            return None;
        }
        let mut out = String::new();
        for (&object, &frame) in self.frames.iter().zip(&self.call_stack).rev() {
            out.push_str(&frame_line(&self.engine.program, object, frame)?);
            out.push('\\');
        }
        out.push_str(&self.runner_frames);
        Some(out)
    }

    /// `ErrorInfo.Callstack` of an error collected now: the live frames, innermost first, a line feed between them, then
    /// `UNTRACKED_CALL_STACK`. The corpus keeps only the lines of its own app after it drops CR and splits at LF
    /// (misc/lt-probes-needed-2/PnBErrorsCollections.al:282-306 and :329-358, RO_CollectedErrorInfo_Internal_And_Client), so
    /// the rest of the text (the test runner's frames, which writer RO saw on BC 27.5 and not on 28.5 or 29.0, CR, a trailing
    /// separator) is not pinned: with the mark the stack is not empty and no text function reads it.
    pub(crate) fn collected_call_stack(&self) -> String {
        if self.events.unpinned_event_depth > 0 {
            return UNTRACKED_CALL_STACK.to_string();
        }
        let mut lines: Vec<String> = self
            .frames
            .iter()
            .zip(&self.call_stack)
            .rev()
            .map(|(&object, &frame)| {
                frame_line(&self.engine.program, object, frame).unwrap_or_else(|| UNTRACKED_CALL_STACK.to_string())
            })
            .collect();
        lines.push(UNTRACKED_CALL_STACK.to_string());
        lines.join("\n")
    }

    /// Adds the line of routine `frame` of `object` to the error's stack.
    pub(crate) fn add_frame(&self, e: &mut AlError, object: usize, frame: CallFrame) {
        if !e.call_stack.is_empty() {
            e.call_stack.push('\n');
        }
        match frame_line(&self.engine.program, object, frame) {
            Some(line) => e.call_stack.push_str(&line),
            None => e.call_stack.push_str(UNTRACKED_CALL_STACK),
        }
    }
}

/// The text of one frame; `None` where the format is not known.
///
/// Measured (session/TestSessionCallstackSourcesContracts.al:Callstack_FromTablePageReportAndExtensionCode,
/// BC 27.5): the object type is `CodeUnit`, `Table`, `Page`, `PageExtension`, `Report` or `ReportExtension`; a
/// trigger of an action or of a report data item is named `"<action or item> - <trigger>"` with `(Trigger)`
/// after it; a procedure with a name that is not a plain identifier is quoted
/// (handlers/PWRefusedBuiltins.al:Callstack_QuotedProcedureName). A trigger inside any other control, query,
/// xmlport and the other object types have not been measured.
fn frame_line(program: &ir::Program, object: usize, frame: CallFrame) -> Option<String> {
    let obj = &program.objects[object];
    let routine = obj.routines.get(frame.routine)?;
    use ir::ObjKind;
    let kind = match obj.kind {
        _ if obj.uncompiled => return None,
        ObjKind::Codeunit => "CodeUnit",
        ObjKind::Table => "Table",
        ObjKind::Page => "Page",
        ObjKind::PageExtension => "PageExtension",
        ObjKind::Report => "Report",
        ObjKind::ReportExtension => "ReportExtension",
        // errors/lt-ms-refusals-128/P128Callstack.al:263-264 (procedures), lt-ms-refusals-129/P129CallstackQuery.al:105
        // (OnBeforeOpen), P129CallstackXmlPort.al:147-153 and lt-ms-refusals-135/P135CallstackXmlportEtc.al:187-189.
        // errors/lt-ms-refusals-129/P129CallstackTableExt.al: the extension's own OnInsert, OnModify, OnDelete and OnRename,
        // an OnValidate of its field and a procedure.
        ObjKind::TableExtension => "TableExtension",
        ObjKind::Query => "Query",
        ObjKind::XmlPort => "XmlPort",
        _ => return None,
    };
    // The triggers of a query or an xmlport the corpus measured; any other is not pinned.
    let pinned_trigger = match obj.kind {
        ObjKind::Query => matches!(
            (routine.container.as_ref().map(|c| c.0.as_str()), routine.name.as_str()),
            (None, "OnBeforeOpen")
        ),
        ObjKind::XmlPort => matches!(
            (routine.container.as_ref().map(|c| c.0.as_str()), routine.name.as_str()),
            (None, "OnPreXmlPort" | "OnPostXmlPort")
                | (Some("tableelement"), "OnPreXmlItem" | "OnAfterGetRecord")
                | (Some("fieldelement"), "OnBeforePassField")
                | (Some("textelement"), "OnBeforePassVariable")
        ),
        ObjKind::TableExtension => match (routine.field, routine.name.as_str()) {
            (Some(_), name) => name == "OnValidate",
            (None, name) => matches!(name, "OnInsert" | "OnModify" | "OnDelete" | "OnRename"),
        },
        _ => true,
    };
    if routine.is_trigger && !pinned_trigger {
        return None;
    }
    // A name with a character other than a letter, digit, underscore, blank or parenthesis is not measured.
    if !routine
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ' ' | '(' | ')'))
    {
        return None;
    }
    // The event frame of a publisher stands at its `end` (corpus
    // session/TestSessionInformationCallstackContracts.al:187: an empty publisher is line 2).
    let stmt = program.pos(if frame.event { routine.end_pos } else { frame.pos })?;
    let signature = program.pos(routine.pos)?;
    let line = stmt.line.checked_sub(signature.line)?;
    let name = match (routine.field, &routine.container) {
        // "<field> - OnValidate" is one quoted name (same file:186).
        (Some(f), _) => {
            // A table extension's field trigger: the field of the extended table.
            let table = match obj.kind {
                ObjKind::TableExtension => program
                    .objects
                    .heads()
                    .iter()
                    .find(|o| o.kind == ObjKind::Table && o.id.is_some() && o.id == obj.extends_id)?
                    .table
                    .as_ref()?,
                _ => obj.table.as_ref()?,
            };
            let field = table.fields.get(f)?;
            format!("\"{} - {}\"", field.name, routine.name)
        }
        (None, Some((section, owner))) if routine.is_trigger => {
            // A page field's OnValidate, OnLookup, OnDrillDown and OnAssistEdit (P128Callstack.al:226-228,
            // P129CallstackPageControls.al:255-261) and the xmlport element triggers checked above.
            let page_field = section == "field"
                && matches!(obj.kind, ObjKind::Page)
                && matches!(
                    routine.name.as_str(),
                    "OnValidate" | "OnLookup" | "OnDrillDown" | "OnAssistEdit"
                );
            let xml_element = matches!(obj.kind, ObjKind::XmlPort)
                && matches!(section.as_str(), "tableelement" | "fieldelement" | "textelement");
            if (matches!(section.as_str(), "action" | "dataitem") || page_field || xml_element)
                && !owner.is_empty()
                && !owner.contains('"')
            {
                format!("\"{owner} - {}\"", routine.name)
            } else {
                return None;
            }
        }
        (None, _) if routine.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') => routine.name.clone(),
        (None, _) => format!("\"{}\"", routine.name),
    };
    let mut out = format!(
        "\"{}\"({kind} {}).{name}{} line {line}",
        obj.name.replace('"', "\"\""),
        obj.id?,
        if frame.event {
            "(Event)"
        } else if routine.is_trigger {
            "(Trigger)"
        } else {
            ""
        },
    );
    if let Some(app) = program.apps.get(obj.app as usize) {
        if !app.publisher.is_empty() && !app.version.is_empty() {
            out.push_str(&format!(" - {} by {} version {}", app.name, app.publisher, app.version));
        }
    }
    Some(out)
}

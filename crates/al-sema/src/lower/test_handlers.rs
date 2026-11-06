//! The ITestHandler codeunits of a test codeunit (BC 29, runtime 18.0): those of the enum extensions of the system enum
//! DefaultTestHandler (2000000024), which apply to every test codeunit, then those its `TestHandlers` property names (values
//! of the system enum TestHandler, 2000000023). Lowering resolves the names; the test runner calls the callbacks
//! (al-testrun `runtime/test_handlers.rs`).
//!
//! Pins: corpus al-language-testlibraries/test-framework/data-driven-tests-and-handlers/BiTestHandlerDefault.al:12-21 (a
//! default handler reaches a codeunit without TestHandlers and one with only data-driven procedures; default handlers run
//! before the TestHandlers handlers), BiTestHandlerBehavior.al:382-441 (two TestHandlers values run in the order listed).

use super::*;
use crate::members::ImplSlot;

use ir::{TEST_HANDLER_CODEUNITS, TEST_HANDLERS_REFUSED};

const TEST_HANDLER_ENUM: u32 = 2000000023;
const DEFAULT_TEST_HANDLER_ENUM: u32 = 2000000024;

/// The synthetic properties of test codeunit `oi` (none for another object, or a test codeunit without handlers).
pub(crate) fn properties(
    ws: &Workspace,
    members: &MemberTable<'_>,
    oi: usize,
    props: &[(String, String)],
) -> Vec<(String, String)> {
    let o = &ws.objects[oi];
    let is_test = props
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"));
    if o.kind != ObjKind::Codeunit || !is_test {
        return Vec::new();
    }
    let own = props
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("TestHandlers"))
        .map(|(_, v)| v.as_str());
    match handlers(ws, members, oi, own) {
        Ok(list) if list.is_empty() => Vec::new(),
        Ok(list) => vec![(TEST_HANDLER_CODEUNITS.into(), list.join(","))],
        Err(why) => vec![(TEST_HANDLERS_REFUSED.into(), why)],
    }
}

fn enum_by_id(ws: &Workspace, id: u32) -> Option<usize> {
    ws.objects
        .iter()
        .position(|o| o.kind == ObjKind::Enum && o.id == Some(id))
}

/// The codeunit id that implements ITestHandler for value `ordinal` of enum `e`.
fn implementation(
    ws: &Workspace,
    members: &MemberTable<'_>,
    e: usize,
    ordinal: u32,
    value: &str,
) -> Result<u32, String> {
    let decl = members[e]
        .enum_impls
        .iter()
        .find(|i| {
            i.slot == ImplSlot::Value(ordinal)
                && i.interface
                    .last()
                    .is_some_and(|n| n.eq_ignore_ascii_case("ITestHandler"))
        })
        .ok_or_else(|| format!("the test handler `{value}` has no ITestHandler implementation"))?;
    let file = ws.objects[decl.declared_in].file;
    match ws.resolve_ref(file, ObjKind::Codeunit, &decl.codeunit) {
        Resolution::Found(c) => ws.objects[c]
            .id
            .ok_or_else(|| format!("the codeunit of test handler `{value}` has no id")),
        _ => Err(format!(
            "the codeunit of test handler `{value}` is not in the compiled apps"
        )),
    }
}

fn handlers(ws: &Workspace, members: &MemberTable<'_>, oi: usize, own: Option<&str>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    if let Some(e) = enum_by_id(ws, DEFAULT_TEST_HANDLER_ENUM) {
        let values = &members[e].enum_values;
        // The order of two default handlers is not pinned (BiTestHandlerDefault.al has one).
        if values.len() > 1 {
            return Err(
                "more than one value of DefaultTestHandler: the order of the default test handlers is not pinned"
                    .into(),
            );
        }
        for (ordinal, name) in values {
            let declared_in = members[e]
                .enum_impls
                .iter()
                .find(|i| i.slot == ImplSlot::Value(*ordinal))
                .map(|i| i.declared_in);
            // A default handler of another app reaches this test codeunit on BC too; it is not probed
            // (BiTestHandlerDefault.al pins handler and test codeunits of one app).
            if declared_in.is_some_and(|d| ws.objects[d].app != ws.objects[oi].app) {
                return Err(format!(
                    "the default test handler `{name}` of another app (DefaultTestHandler for the test codeunits of other apps is not pinned)"
                ));
            }
            out.push(format!("D{}", implementation(ws, members, e, *ordinal, name)?));
        }
    }
    let Some(own) = own else {
        return Ok(out);
    };
    let e = enum_by_id(ws, TEST_HANDLER_ENUM).ok_or("the system enum TestHandler is not in the compiled apps")?;
    for raw in own.split(',') {
        let name = raw.trim().trim_matches('"');
        let (ordinal, value) = members[e]
            .enum_values
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("TestHandlers names `{name}`, which is not a value of the enum TestHandler"))?;
        let entry = format!("O{}", implementation(ws, members, e, *ordinal, value)?);
        // The same codeunit twice in one codeunit's handlers (as a default and an own handler, or two values) is not pinned.
        if out.iter().any(|o| o[1..] == entry[1..]) {
            return Err(format!(
                "the test handler codeunit of `{name}` is called twice for one test codeunit (not pinned)"
            ));
        }
        out.push(entry);
    }
    Ok(out)
}

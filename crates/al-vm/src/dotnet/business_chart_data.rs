//! `Microsoft.Dynamics.Nav.Client.BusinessChart.BusinessChartData` (ADR-014 section 1): the chart model System
//! Application's "Business Chart Impl." keeps behind Base Application's "Business Chart Buffer". It is plain data with no
//! session, but its assembly is a server class (`server-prefix Microsoft.Dynamics.Nav.`), so the registry serves it.
//!
//! Pinned by tests/al-language/misc/lt-au-language-edges/AuEdgeTests.al:608 (BusinessChartBuffer_Initialize_InTest, real BC
//! 27-29): Initialize constructs it and calls `ClearMeasures`, SetXAxis sets `XDimension`, AddIntegerMeasure calls
//! `AddMeasure(Caption, DataMeasureType)`, and GetXCaption reads `XDimension` back ('Month').
//!
//! Refused: every other member. `ShowChartCondensed` and `DataTable` are only seen through `Update`, which serializes the
//! object for the BusinessChart add-in; no test pins that JSON, so neither is served.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::BUSINESS_CHART_DATA;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language/misc/lt-au-language-edges/AuEdgeTests.al:608-631";

/// State slots: the x dimension caption, and how many measures were added since the last `ClearMeasures`.
const X_DIMENSION: usize = 0;
const MEASURES: usize = 1;

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn state(this: Option<&Rc<DotNetObject>>) -> Result<&RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name, state }) if type_name == BUSINESS_CHART_DATA => Ok(state),
        _ => unsupported(format!(
            "DotNet {BUSINESS_CHART_DATA} member on something that is not a {BUSINESS_CHART_DATA} the runtime made"
        )),
    }
}

fn construct(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet new {BUSINESS_CHART_DATA} with arguments (not pinned)"));
    }
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: BUSINESS_CHART_DATA.into(),
        state: RefCell::new(vec![Value::Text("".into()), Value::Integer(0)]),
    }))))
}

fn clear_measures(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet {BUSINESS_CHART_DATA}.ClearMeasures takes no arguments"));
    }
    state(this)?.borrow_mut()[MEASURES] = Value::Integer(0);
    Ok(Value::Null)
}

fn add_measure(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [Value::Text(_), _] = args else {
        return unsupported(format!(
            "DotNet {BUSINESS_CHART_DATA}.AddMeasure takes a caption and a measure type here"
        ));
    };
    let mut st = state(this)?.borrow_mut();
    if let Value::Integer(n) = &mut st[MEASURES] {
        *n += 1;
    }
    Ok(Value::Null)
}

fn x_dimension_get(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet {BUSINESS_CHART_DATA}.XDimension takes no arguments"));
    }
    Ok(state(this)?.borrow()[X_DIMENSION].clone())
}

fn x_dimension_set(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [Value::Text(caption)] = args else {
        return unsupported(format!(
            "DotNet {BUSINESS_CHART_DATA}.XDimension set to something that is not a Text"
        ));
    };
    state(this)?.borrow_mut()[X_DIMENSION] = Value::Text(caption.clone());
    Ok(Value::Null)
}

fn member(name: &str, kind: MemberKind, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: Pin::Corpus(PIN.into()),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    use MemberKind::{Call, Construct, Get, Set};
    vec![RegistryType {
        clr_name: BUSINESS_CHART_DATA.into(),
        members: vec![
            member("BusinessChartData", Construct, construct),
            member("ClearMeasures", Call, clear_measures),
            member("AddMeasure", Call, add_measure),
            member("XDimension", Get, x_dimension_get),
            member("XDimension", Set, x_dimension_set),
        ],
    }]
}

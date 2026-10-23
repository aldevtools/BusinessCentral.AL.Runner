//! `System.Collections.Generic.Dictionary`2` (ADR-014 section 1). AL declares it without type arguments, an open generic type
//! the .NET host cannot create. Base Application's ODataUtility.GenerateFilterText creates one, fills it in GetNAVFilters
//! from Tenant Web Service Filter rows only, and walks it with `foreach` in CombineFiltersFromTables. With no filter rows
//! the dictionary stays empty, the walk finds nothing and the V3 and V4 filter texts are '' (tests/al-language-onprem-dotnet/
//! lt-qx-dotnet/QxDotNetDemoShapes.al:473, a query service; al-language-onprem/platform/PtWebServiceRows.al:326, a page
//! service; Microsoft's DemoTool, Create Web Services).
//!
//! Served: the constructor without arguments, and `foreach` over the (empty) dictionary (`mod.rs`). Refused: every other
//! member (`Add` among them: no corpus test has a filter row, so what the dictionary holds then is not pinned).

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{GENERIC_DICTIONARY, GENERIC_DICTIONARY_ENUMERATOR};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:473; \
    al-language-onprem/platform/PtWebServiceRows.al:326";

fn new_dictionary(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return Err(Signal::Unsupported(format!(
            "DotNet new {GENERIC_DICTIONARY} with arguments (only the constructor without arguments is pinned)"
        )));
    }
    Ok(object(GENERIC_DICTIONARY))
}

fn object(type_name: &str) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(Vec::new()),
    })))
}

/// The enumerator `foreach` steps over a dictionary the runtime made, if `type_name` names one. It is always empty: no
/// member that adds an entry is served.
pub(crate) fn foreach_start(type_name: &str) -> Option<Value> {
    (type_name == GENERIC_DICTIONARY).then(|| object(GENERIC_DICTIONARY_ENUMERATOR))
}

/// Whether `type_name` is the enumerator of [`foreach_start`]: the walk ends at once.
pub(crate) fn is_enumerator(type_name: &str) -> bool {
    type_name == GENERIC_DICTIONARY_ENUMERATOR
}

pub fn types() -> Vec<RegistryType> {
    vec![RegistryType {
        clr_name: GENERIC_DICTIONARY.into(),
        members: vec![RegistryMember {
            name: "Dictionary".into(),
            kind: MemberKind::Construct,
            pin: Pin::Corpus(PIN.into()),
            call: new_dictionary,
            call_ref: None,
        }],
    }]
}

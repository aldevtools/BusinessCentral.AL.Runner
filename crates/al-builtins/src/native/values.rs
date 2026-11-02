use super::*;

pub(super) fn abs(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    Ok(match &args[0] {
        Value::Decimal(d) => Value::Decimal(d.abs()),
        // Abs keeps the type of its argument: `V := Abs(I)` holds an Integer and `Abs(Bi)` a BigInteger
        // (types/lt-variant-is-text-streams/LTP89IsMisc.al:NearTypes_TextVsCode_IntegerVsBigInteger `abs_int`, `abs_bigint`).
        Value::Integer(i) => match i.checked_abs() {
            Some(n) => Value::Integer(n),
            // types/lt-scalars/ScT70056.al:Integer_Overflow `abs_min`.
            None => {
                return raise(
                    "Negating the minimum value of a twos complement number is invalid.",
                    "AL:Overflow",
                );
            }
        },
        Value::BigInteger(i) => match i.checked_abs() {
            Some(n) => Value::BigInteger(n),
            None => return unsupported("Abs of the smallest BigInteger (the overflow error is not pinned)"),
        },
        // A Duration takes the BigInteger overload: its milliseconds (misc/lt-protected-var-abs-query-views/P146AbsDuration.al,
        // P146AbsDurationBigInt.al).
        other => return abs_other(other),
    })
}

/// `Abs` of a Duration is the BigInteger of its milliseconds: `Format(Abs(D))` is `1500`, assigned to a Decimal it is `1,500`
/// (misc/lt-protected-var-abs-query-views/P146AbsDuration.al:P146_Abs_Duration_ResultFormats, P146AbsDurationBigInt.al).
#[cold]
#[inline(never)]
pub(super) fn abs_other(v: &Value) -> Result<Value, Signal> {
    match v {
        Value::Duration(ms) => Ok(Value::BigInteger(ms.abs())),
        other => unsupported(&format!("Abs of {}", other.describe())),
    }
}

/// `Variant2Date(Variant)`: the Date a Variant holds (session/TestFinalCoverage.al:79-88). Another
/// content raises a conversion error whose text is not pinned: refused.
pub(super) fn variant_2_date(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(d @ Value::Date(..)) => Ok(d.clone()),
        other => unsupported(&format!(
            "Variant2Date of {other} (the conversion error is not pinned)",
            other = other.describe()
        )),
    }
}

/// `Variant2Time(Variant)`: the Time a Variant holds (session/TestFinalCoverage.al:91-100).
pub(super) fn variant_2_time(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(t @ Value::Time(_)) => Ok(t.clone()),
        other => unsupported(&format!(
            "Variant2Time of {other} (the conversion error is not pinned)",
            other = other.describe()
        )),
    }
}

/// `Clear(var X)`: the default of the value's own kind. Records go to the VM.
pub(super) fn clear(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let v = &mut args[0];
    *v = match v {
        Value::Record(_) => return ctx.record(RecordOp::Clear, Some(v), &mut []),
        // Clearing a RecordRef/FieldRef/KeyRef closes it: a fresh, unopened handle.
        Value::Library(name, _)
            if matches!(
                name.as_str(),
                "RecordRef"
                    | "FieldRef"
                    | "KeyRef"
                    | "RecordId"
                    | "WebServiceActionContext"
                    | "SessionSettings"
                    | "Notification"
                    // A cleared XML document, document type or declaration is an unassigned one (xml/lt-xml-doctype-declaration/
                    // DocTypeErrors.al:Uninitialised_*, DocTypeRead.al: `Clear(Doc)`).
                    | "XmlDocument"
                    | "XmlDocumentType"
                    | "XmlDeclaration"
                    // types/TestClearContracts.al:Clear_TextBuilder_ResetsToEmpty.
                    | "TextBuilder"
                    | "Cookie"
                    | "JsonObject"
                    | "JsonArray"
                    | "JsonValue"
                    | "JsonToken"
            ) =>
        {
            Value::Library(name.clone(), Default::default())
        }
        // Clear empties the controls and leaves an assigned page caption alone
        // (filterpage/TestFilterPageBuilder.al:FilterPageBuilder_Clear_KeepsTheAssignedCaption).
        Value::Library(crate::library_kind::LibraryKind::FilterPageBuilder, cell) => {
            let caption = match &*cell.borrow() {
                crate::value::LibraryValue::FilterPageBuilder(s) => s.caption.clone(),
                _ => None,
            };
            let state = crate::value::LibraryValue::FilterPageBuilder(crate::value::FilterPageState {
                controls: Vec::new(),
                caption,
            });
            Value::Library(
                crate::library_kind::LibraryKind::FilterPageBuilder,
                std::rc::Rc::new(std::cell::RefCell::new(state)),
            )
        }
        Value::Boolean(_) => Value::Boolean(false),
        Value::Integer(_) => Value::Integer(0),
        Value::BigInteger(_) => Value::BigInteger(0),
        Value::Decimal(_) => Value::Decimal(Default::default()),
        Value::Byte(_) => Value::Byte(0),
        Value::Char(_) => Value::Char(0),
        Value::Text(_) => Value::Text(String::new().into()),
        Value::Code(_) => Value::Code(String::new().into()),
        Value::Date(..) => Value::Date(0, false),
        Value::Time(_) => Value::Time(0),
        Value::DateTime(_) => Value::DateTime(0),
        Value::Duration(_) => Value::Duration(0),
        Value::Guid(_) => Value::Guid([0; 16]),
        // The empty formula (types/TestDateFormulaDurationFormatContracts.al:38-42).
        Value::DateFormula(_) => Value::DateFormula(String::new()),
        Value::Option(_, m) => Value::Option(0, m.clone()),
        // A cleared Enum is the enum's initial value, ordinal 0, as a variable that was never assigned (`default_for`;
        // types/TestBlankEnumInitTests.Codeunit.al for the blank member reading as ordinal 0).
        Value::Enum(id, _) => Value::Enum(*id, 0),
        // A cleared List or Dictionary is a new, empty one (types/TestClearContracts.al:130-150,
        // collections/TestListByRefContracts.al:155 through a `var` parameter).
        Value::List(_) => Value::List(Default::default()),
        Value::Dictionary(_) => Value::Dictionary(Default::default()),
        // A cleared Blob is a new, empty buffer (streams/TestBlobCalcFieldsUncommittedWrite.al).
        Value::Blob(_) => Value::Blob(crate::stream::new_blob()),
        // A cleared TestPage or Page variable is a fresh, closed one (corpus
        // handlers/TestPageEditable_Tests.al:93 EditableBoundToPageVariable_FollowsTheRow). A page
        // that is open, waits for Trap(), or is a part is not cleared: nothing pins what follows.
        Value::TestPage(st) => {
            let object = {
                let s = st.borrow();
                if s.open || s.trapping || s.host.is_some() {
                    return unsupported("Clear of a TestPage that is open, trapping or a part (not pinned)");
                }
                s.object
            };
            Value::TestPage(std::rc::Rc::new(std::cell::RefCell::new(
                crate::value::PageState::closed(object),
            )))
        }
        // A cleared Query is a fresh one: no filters, no row cap, not open
        // (query/TestQueryRuntimeContracts.al:Query_GetFilters_ThreeFilters_OrderIsNotColumnOrder).
        Value::Query(q) => {
            let object = q.borrow().object;
            Value::Query(Rc::new(std::cell::RefCell::new(crate::value::QueryState {
                object,
                filters: Vec::new(),
                filter_replaced: false,
                top: None,
                security_filtering: None,
                run: None,
                instance: None,
                in_before_open: false,
                closed: false,
            })))
        }
        // A codeunit or report variable nothing has used yet has no instance to release, so
        // clearing it changes nothing (handlers/TestReportRunWithRequestPage.al: `Clear(Probe)`
        // before `Probe.Run()`). Clearing one in use gives a fresh instance: after `Clear(TempBlob)` the
        // blob is empty again (streams/TestStreamsRuntimeContracts.al:WriteText_ExplicitEncodings_Bytes).
        Value::Codeunit(rc) if !rc.borrow().initialized => return Ok(Value::Null),
        Value::Codeunit(rc) => {
            let object = rc.borrow().object;
            Value::Codeunit(std::rc::Rc::new(std::cell::RefCell::new(crate::value::Instance {
                object,
                ..Default::default()
            })))
        }
        // An array goes back to its initial value element by element, in place: a `var` parameter
        // that shares it sees the change (docs/probes-needed.md; the oracle runs
        // pageupdate/PageUpdateFacts.Page.al:58 `Clear(Derived)` on an array of Decimal).
        Value::Array(a) => {
            let a = a.clone();
            // An array of JsonObject keeps every element as it was (json/lt-db-json-leads/DbJsonLeads.al:150-164). An array
            // of another JSON type is not pinned.
            match a.borrow().first() {
                Some(Value::Library(name, _)) if name.as_str() == "JsonObject" => return Ok(Value::Null),
                Some(Value::Library(name, _)) if matches!(name.as_str(), "JsonArray" | "JsonValue" | "JsonToken") => {
                    return unsupported(&format!("Clear of an array of {} (not pinned)", name.as_str()));
                }
                _ => {}
            }
            for e in a.borrow_mut().iter_mut() {
                clear(ctx, None, std::slice::from_mut(e))?;
            }
            return Ok(Value::Null);
        }
        // An unassigned DotNet variable holds nothing, so clearing it changes nothing. One that holds
        // an object is refused: whether BC calls Dispose on it, and so whether a stream the object
        // wraps is flushed, is not pinned (docs/probes-needed.md, "Clear of a DotNet variable").
        Value::Null => return Ok(Value::Null),
        // The App Key Vault provider holds nothing and wraps no stream, so a Dispose would show nothing: Clear of a
        // constructed one gives an unassigned variable (onprem-dotnet/lt-ty-dotnet/TyAkvProvider.al:95, :114, :134,
        // `Clear(Provider)` at the start of each test, after the one before assigned it).
        Value::DotNet(o)
            if matches!(&o.kind, crate::value::DotNetKind::Internal { type_name, .. }
                if type_name == crate::dotnet_sigs::AKV_PROVIDER) =>
        {
            Value::Null
        }
        // The Key Vault client and Microsoft's test provider hold names and values only: "Azure Key Vault Impl.".ClearSecrets
        // clears both and the next read builds a new client (onprem-dotnet/lt-y58-key-vault/Y58AzureKeyVault.al:245
        // KeyVault_Cache, 'read after ClearSecrets' = not set up; corpus master 7330f181).
        Value::DotNet(o)
            if matches!(&o.kind, crate::value::DotNetKind::Internal { type_name, .. }
                if type_name == crate::dotnet_sigs::KEY_VAULT_HELPER || type_name == crate::dotnet_sigs::KEY_VAULT_MOCK_PROVIDER) =>
        {
            Value::Null
        }
        // Excel Buffer.CloseBook clears its worksheet and workbook writers after the workbook's Close, and its worksheet and
        // workbook readers: the variables become unassigned (assumptions clear-closed-workbook-writer and clear-workbook-reader,
        // from Microsoft's Tests-ERM; the context marks them). A later call on the object through another
        // variable is refused (the context remembers the handle).
        Value::DotNet(o)
            if matches!(&o.kind, crate::value::DotNetKind::Host { handle, clr_type }
                if ctx.dotnet_clear_is_inert(*handle, clr_type)) =>
        {
            Value::Null
        }
        Value::DotNet(o) => {
            let ty = match &o.kind {
                crate::value::DotNetKind::Host { clr_type, .. } => clr_type.as_str(),
                crate::value::DotNetKind::Enum { clr_type, .. } => clr_type.as_str(),
                crate::value::DotNetKind::Internal { type_name, .. } => type_name.as_str(),
            };
            return unsupported(&format!(
                "Clear of a DotNet variable that holds a {ty}: whether BC disposes the object is not pinned"
            ));
        }
        // A cleared stream is one that was never created again (streams/TestProbeStreamContracts.al:RecordAssign_CopiesBlobBytes).
        // A cleared SecretText is an empty one and a copy taken before keeps its content
        // (fieldref/lt-fieldref-testfield-types/SystemClearArraySecret.al:69-90, errorinfo/lt-errnotif/EnTSecret.al:15-26).
        // A cleared ModuleInfo reads as one nobody filled (platform/lt-platform-members/PlTNavApp.al:69-81).
        // A cleared BigText is empty (streams/StTBigText.al:ClearBigText `len-after-clear` 0, `len-after-add` 1).
        // A cleared HttpClient is a new one: no base address, the default timeout, no default headers; the next call
        // starts it fresh (network/lt-http-relative-request/HttpRelativeRequestPath.al:NewClient clears, then sets the base address).
        // A cleared ErrorInfo is a fresh one, and so is every variable assigned from it, which shares it: `Error` of it raises ''
        // with code Dialog (misc/lt-system-functions/ErrorMessage.al:184-206), its SystemId is the empty Guid
        // (errorinfo/lt-bc-errorinfo-systemid/BcErrorInfoSystemId.al:55-66), TestField given it raises the plain text
        // (record/lt-testfield-value-types/RecordTestFieldGroupA.al:59-72), and `B := A; Clear(A)` empties B's Message too
        // (errorinfo Copy_Semantics, codeunit 73026).
        Value::Library(name, cell) if name.as_str() == "ErrorInfo" => {
            *cell.borrow_mut() = crate::value::LibraryValue::Empty;
            return Ok(Value::Null);
        }
        // A cleared HttpRequestMessage, HttpContent or HttpResponseMessage is a new one, and a value assigned from it before keeps
        // what it held: the request is GET with no URI, no headers and no content (NotInitialized on ReadAs); the content is empty,
        // with only the Content-Type text/plain; the response is 200 OK with an empty body (network/lt-uf-http-clear/UfHttpClear.al:
        // 14-62, :64-114, :116-145; al-runner issues 6170, 6171).
        Value::Library(name, _)
            if matches!(
                name.as_str(),
                "HttpRequestMessage" | "HttpContent" | "HttpResponseMessage"
            ) =>
        {
            Value::Library(
                name.clone(),
                std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Empty)),
            )
        }
        // A cleared TableFilter field is the empty filter, as on a fresh record; the other fields keep their values and the stored
        // row changes only with Modify (record/lt-wc/WcClearTableFilter.al:36-90, al-runner issue 6318).
        Value::Library(name, _) if name.as_str() == "TableFilter" => Value::Library(
            name.clone(),
            std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Empty)),
        ),
        Value::Library(name, _)
            if matches!(
                name.as_str(),
                "OutStream"
                    | "InStream"
                    | "FileUpload"
                    | "HttpClient"
                    | "HttpHeaders"
                    | "SecretText"
                    | "ModuleInfo"
                    | "BigText"
            ) =>
        {
            Value::Library(
                name.clone(),
                std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Empty)),
            )
        }
        // A cleared XmlElement is an unassigned one and a cleared XmlNode an empty one; another variable that holds the same
        // node keeps it, and the node stays in its document (corpus branch al-runtime-rs-staging-ul b753d3fb7a,
        // xml/lt-ul-clear-ref/UlClearXmlRef.al:53-86 ClearXmlElement_SecondVariable, :106-142 ClearXmlNode_SecondVariable).
        Value::Library(name, _) if matches!(name.as_str(), "XmlElement" | "XmlNode") => {
            Value::Library(name.clone(), Default::default())
        }
        // A cleared XmlNodeList is an empty one, a copy taken before keeps its nodes, and SelectNodes fills it again
        // (xml/XmlNodes.al TP_NodeList_Clear).
        Value::Library(name, _) if name.as_str() == "XmlNodeList" => Value::Library(
            name.clone(),
            std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::XmlList(Vec::new()))),
        ),
        Value::Library(name, _) if name.as_str() == "Dialog" => dialog::cleared(name),
        other => return unsupported(&format!("Clear of a {} value", value_kind(other))),
    };
    Ok(Value::Null)
}

pub(super) fn is(receiver: Option<&mut Value>, kind: &str) -> Result<Value, Signal> {
    Ok(Value::Boolean(receiver.is_some_and(|v| v.is(kind))))
}

/// The kind of a value for a refusal reason: the variant name, never its contents.
pub(super) fn value_kind(v: &Value) -> String {
    let d = v.describe();
    d.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()
}

/// `<Date or Boolean>.ToText()` as `Format(value)` (assumption `primitive-totext-is-format`).
/// Case mapping one character to one (types/lt-scalars/ScT70509.al:LowerUpperBasic `a9`, LowerUpperLengthsAndTargets `lenu`): an
/// upper-case `ß` stays `ß` and the dotted capital I is a plain `i`.
pub(crate) fn simple_upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

/// `Text.ToUpper()` / `ToLower()`: one character to one, and a character the mapping would turn into several stays (`ß`, `ﬁ`, the dotted
/// capital `İ`), the dotless `ı` stays too (text/lt-text-instance/CaseTrim1.Codeunit.al:ToLower_ToUpper_Unicode; unlike `UpperCase`/`LowerCase`).
pub(crate) fn method_upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == 'ı' {
                return c;
            }
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

pub(crate) fn method_lower(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut low = c.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        })
        .collect()
}

pub(crate) fn simple_lower(s: &str) -> String {
    s.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect()
}

pub(super) fn guid_to_text(r: Option<&mut Value>) -> Result<Value, Signal> {
    match r.as_deref() {
        Some(Value::Guid(g)) => Ok(Value::Text(crate::format::format_guid(g).into())),
        other => Err(Signal::Unsupported(format!("Guid.ToText on {}", other.describe()))),
    }
}

pub(super) fn byte_to_text(c: &mut dyn Ctx, r: Option<&mut Value>) -> Result<Value, Signal> {
    match r.as_deref() {
        Some(v @ Value::Byte(_)) => match crate::format::try_format(v, 0, 0, &*c) {
            Ok(t) => Ok(Value::Text(t.into())),
            Err(e) => Err(Signal::Unsupported(e)),
        },
        other => Err(Signal::Unsupported(format!("Byte.ToText on {}", other.describe()))),
    }
}

pub(super) fn primitive_to_text(
    c: &mut dyn Ctx,
    r: Option<&mut Value>,
    what: &str,
    a: &[Value],
) -> Result<Value, Signal> {
    match r.as_deref() {
        // `Date.ToText(true)` is the round-trip form `yyyy-mm-dd`, `ToText(false)` the default one; the first of January 1 has
        // no text (types/lt-totext-bigtext-codeunit-run/P93Date.al:ToText_Normal, ToText_Closing).
        Some(v @ Value::Date(..)) => {
            let iso = matches!(a.first(), Some(Value::Boolean(true)));
            if matches!(v, Value::Date(1, _)) {
                return raise("The date is not valid.", "DateInvalid");
            }
            match crate::format::try_format(v, 0, if iso { 9 } else { 0 }, &*c) {
                Ok(t) => Ok(Value::Text(t.into())),
                Err(e) => Err(Signal::Unsupported(e)),
            }
        }
        // The text of a Time, DateTime, Duration, Boolean and Decimal is `Format` with the default format, and with `true` the
        // round-trip form (Format standard 9): `13:45:01`, `2024-03-15T13:45:01Z`, `true`, `1234567.891`, and the Duration words
        // `1 hour 2 minutes 3 seconds` (types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:131-135, 156-160, 175-190,
        // 237-253).
        Some(
            v @ (Value::Time(_) | Value::DateTime(_) | Value::Duration(_) | Value::Boolean(_) | Value::Decimal(_)),
        ) => {
            let inv = matches!(a.first(), Some(Value::Boolean(true)));
            match crate::format::try_format(v, 0, if inv { 9 } else { 0 }, &*c) {
                Ok(t) => Ok(Value::Text(t.into())),
                Err(e) => Err(Signal::Unsupported(e)),
            }
        }
        other => Err(Signal::Unsupported(format!("{what}.ToText on {}", other.describe()))),
    }
}

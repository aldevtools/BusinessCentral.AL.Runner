//! The handler functions a test declares, bound for the length of the test.
use super::*;

/// The names a `[HandlerFunctions('A,B')]` attribute lists, or `None` when its argument is no text.
fn handler_names(attr: &al_ir::Attribute) -> Option<Vec<String>> {
    match attr.args.as_slice() {
        [al_ir::Const::Text(list)] => Some(list.split(',').map(|n| n.trim().to_string()).collect()),
        _ => None,
    }
}

/// The routines test codeunit `oi` starts AL code from: its `[Test]` methods, its `OnRun` trigger with `onrun`, and the
/// handler functions its tests name in `[HandlerFunctions]`, which the test runner calls while the test runs. The one root
/// list of a test codeunit's static reach (`chain.rs`, `corpus.rs` `reaches_session_storage`, `strict_taint.rs`). A name
/// that is no routine of the codeunit adds nothing: `resolve_handlers` refuses that test.
pub(crate) fn unit_roots(lowered: &Lowered, oi: usize, onrun: bool) -> Vec<(usize, usize)> {
    let o = &lowered.program.objects[oi];
    let is_test = |r: &al_ir::Routine| r.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test"));
    let mut roots: Vec<usize> = (0..o.routines.len())
        .filter(|&r| {
            let routine = &o.routines[r];
            is_test(routine) || (onrun && routine.is_trigger && routine.name.eq_ignore_ascii_case("OnRun"))
        })
        .collect();
    let named: Vec<String> = o
        .routines
        .iter()
        .filter(|r| is_test(r))
        .flat_map(|r| {
            r.attributes
                .iter()
                .filter(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
        })
        .filter_map(handler_names)
        .flatten()
        .collect();
    for name in named {
        if let Some(hr) = o.routines.iter().position(|r| r.name.eq_ignore_ascii_case(&name)) {
            if !roots.contains(&hr) {
                roots.push(hr);
            }
        }
    }
    roots.into_iter().map(|r| (oi, r)).collect()
}

/// The handler functions test routine `test` of `object` declares with
/// `[HandlerFunctions('A,B')]`, as (kind, routine index), or why the test
/// cannot run with them. Handlers whose kind is not implemented are refused
/// here, by name and kind (hard rule 4).
pub(crate) fn resolve_handlers(lowered: &Lowered, object: usize, test: usize) -> Result<Vec<(UiKind, usize)>, String> {
    let obj = &lowered.program.objects[object];
    let mut out: Vec<(UiKind, usize)> = Vec::new();
    // Why handler routine `r` cannot run: code reachable from it is refused.
    let handler_blocked = |r: usize| {
        blocked_reason(lowered, object, r).map(|reason| format!("handler {}: {reason}", obj.routines[r].name))
    };
    for attr in obj.routines[test]
        .attributes
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
    {
        let Some(names) = handler_names(attr) else {
            return Err(format!("{} is not a list of handler names", attr.text));
        };
        for name in names {
            let Some(hr) = obj.routines.iter().position(|r| r.name.eq_ignore_ascii_case(&name)) else {
                return Err(format!("handler function {name} not found in {}", obj.name));
            };
            if out.iter().any(|(_, r)| *r == hr) {
                continue;
            }
            let routine = &obj.routines[hr];
            let kind_attr =
                routine.attributes.iter().map(|a| a.name.as_str()).find(|n| {
                    n.to_ascii_lowercase().ends_with("handler") && !n.eq_ignore_ascii_case("HandlerFunctions")
                });
            let Some(kind_attr) = kind_attr else {
                return Err(format!(
                    "{} is named in [HandlerFunctions] but is no handler function",
                    routine.name
                ));
            };
            let Some(kind) = UiKind::from_attribute(kind_attr) else {
                return Err(format!(
                    "handler {} is a [{kind_attr}]: this handler kind, which is not implemented yet",
                    routine.name
                ));
            };
            let page_of = |r: &al_ir::Routine| match r.locals.first().map(|l| &l.ty) {
                Some(
                    al_builtins::value::RtType::TestPage(p)
                    | al_builtins::value::RtType::Page(p)
                    | al_builtins::value::RtType::Codeunit(p),
                ) => Some(*p),
                _ => None,
            };
            let is_page = matches!(
                kind,
                UiKind::Page | UiKind::ModalPage | UiKind::RequestPage | UiKind::Report
            );
            if let Some((_, other)) = out
                .iter()
                .find(|(k, r)| *k == kind && (!is_page || page_of(&obj.routines[*r]) == page_of(routine)))
            {
                // A page handler whose page type was not lowered (its parameter is refused, so `page_of` is `None`) is
                // refused for that reason, as with one handler (al-runner issue 8100), not as a binding to choose.
                if is_page {
                    for r in [*other, hr] {
                        if page_of(&obj.routines[r]).is_none()
                            && let Some(why) = handler_blocked(r)
                        {
                            return Err(why);
                        }
                    }
                }
                return Err(format!(
                    "{} and {} are both {kind:?} handlers; which one BC would bind is not pinned",
                    obj.routines[*other].name, routine.name
                ));
            }
            out.push((kind, hr));
        }
    }
    // Code reachable from a handler must be runnable too (decided before running).
    for (_, hr) in &out {
        if let Some(why) = handler_blocked(*hr) {
            return Err(why);
        }
    }
    Ok(out)
}

/// Binds the resolved handlers to the test codeunit's instance for one test method.
pub(crate) fn bind_handlers(
    session: &mut Session,
    instance: &std::rc::Rc<std::cell::RefCell<al_builtins::value::Instance>>,
    handlers: &[(UiKind, usize)],
) -> Result<(), String> {
    let object = instance.borrow().object as usize;
    // TestHttpRequestPolicy is a property of the test codeunit; absent, outbound requests are allowed.
    let policy = session.object_property(object, "TestHttpRequestPolicy");
    session.set_http_policy(policy.as_deref());
    let mut bound = Vec::new();
    for (kind, routine) in handlers {
        bound.push(session.ui_handler(*kind, object, *routine, instance.clone())?);
    }
    session.begin_ui_scope(bound);
    Ok(())
}

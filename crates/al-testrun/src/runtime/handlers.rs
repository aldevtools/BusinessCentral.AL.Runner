//! The handler functions a test declares, bound for the length of the test.
use super::*;

/// The handler functions test routine `test` of `object` declares with
/// `[HandlerFunctions('A,B')]`, as (kind, routine index), or why the test
/// cannot run with them. Handlers whose kind is not implemented are refused
/// here, by name and kind (hard rule 4).
pub(crate) fn resolve_handlers(lowered: &Lowered, object: usize, test: usize) -> Result<Vec<(UiKind, usize)>, String> {
    let obj = &lowered.program.objects[object];
    let mut out: Vec<(UiKind, usize)> = Vec::new();
    for attr in obj.routines[test]
        .attributes
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
    {
        let names: Vec<String> = match attr.args.as_slice() {
            [al_ir::Const::Text(list)] => list.split(',').map(|n| n.trim().to_string()).collect(),
            _ => return Err(format!("{} is not a list of handler names", attr.text)),
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
                let why = match kind_attr.to_ascii_lowercase().as_str() {
                    "filterpagehandler" => {
                        "no corpus test pins how FilterPageBuilder.RunModal reaches a filter page handler"
                    }
                    _ => "this handler kind",
                };
                return Err(format!(
                    "handler {} is a [{kind_attr}]: {why}, which is not implemented yet",
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
        if let Some(reason) = blocked_reason(lowered, object, *hr) {
            return Err(format!("handler {}: {reason}", obj.routines[*hr].name));
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

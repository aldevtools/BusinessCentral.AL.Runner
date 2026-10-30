//! XmlPort Metadata and Query Metadata: one row per xmlport or query object, from the object's own
//! declaration (shells included), keyed by object id.
//!
//! Pinned by xmlport/TestXmlPortMetadataVirtualTable.al and query/TestQueryMetadataVirtualTable.al.
//! A property the object does not declare reads its documented default; those defaults are not
//! pinned by the corpus and are recorded in docs/probes-needed.md.

use super::mat::Entry;
use super::objects::{Catalog, caption, entries, ident, member, prop, string_lit, truncate, truthy};
use super::{Providers, blank, option, pos};
use al_builtins::value::Value;
use al_data::TableDef;
use al_ir::{Object, ObjectHead, ObjectInfo};

/// An option property as its ordinal: undeclared reads `default`, an unknown member is refused.
fn ordinal<O: ObjectInfo + ?Sized>(o: &O, name: &str, members: &[&str], default: i32) -> Result<i32, String> {
    match prop(o, name) {
        None => Ok(default),
        Some(v) => member(members, v)
            .and_then(|m| members.iter().position(|x| x == &m))
            .map(|i| i as i32)
            .ok_or_else(|| format!("{} '{}': {name} `{v}` is not understood", o.kind(), o.name())),
    }
}

fn text<O: ObjectInfo + ?Sized>(o: &O, name: &str, max: usize) -> String {
    prop(o, name)
        .and_then(string_lit)
        .map(|s| truncate(&s, max))
        .unwrap_or_default()
}

const TRANSACTION_TYPES: &[&str] = &["UpdateNoLocks", "Update", "Snapshot", "Browse", "Report"];
const DIRECTIONS: &[&str] = &["Import", "Export", "Both"];
const TEXT_ENCODINGS: &[&str] = &["MSDos", "UTF8", "UTF16", "Windows"];
const FORMATS: &[&str] = &["Xml", "VariableText", "FixedText"];

/// XmlPort Metadata: ID (1), Name (2), Caption (3), UseRequestPage (4), TransactionType (5),
/// Direction (6), Encoding (7), TextEncoding (8), Format (9), "App ID" (10).
pub(super) fn xmlport_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    entries(
        &c,
        t,
        |o| match (o.id, o.kind) {
            (Some(id), al_ir::ObjKind::XmlPort) => Some(vec![(1, Value::Integer(id as i32))]),
            _ => None,
        },
        |o, r| {
            r[pos(t, 2)] = Value::Text(truncate(&o.name, 30).into());
            r[pos(t, 3)] = Value::Text(truncate(&caption(o), 80).into());
            r[pos(t, 4)] = Value::Boolean(prop(o, "UseRequestPage").is_none_or(truthy));
            r[pos(t, 5)] = option(t, 5, ordinal(o, "TransactionType", TRANSACTION_TYPES, 0)?);
            r[pos(t, 6)] = option(t, 6, ordinal(o, "Direction", DIRECTIONS, 2)?);
            // A left-out `Encoding` reads UTF16 (xmlport/TestPlatformMetadataDefaultsContracts.al:93). A
            // declared one reads the member of the column that it names; a name the column lacks is not
            // pinned.
            let Value::Option(_, members) = option(t, 7, 0) else {
                unreachable!("an option")
            };
            let encoding = match prop(o, "Encoding") {
                None => "UTF16".to_string(),
                Some(v) => ident(v),
            };
            let at = members.iter().position(|m| m.eq_ignore_ascii_case(&encoding)).ok_or_else(|| {
                format!(
                    "xmlport '{}': the Encoding column has no member `{encoding}`, and what it reads for it is not pinned",
                    o.name
                )
            })?;
            r[pos(t, 7)] = option(t, 7, at as i32);
            r[pos(t, 8)] = option(t, 8, ordinal(o, "TextEncoding", TEXT_ENCODINGS, 1)?);
            r[pos(t, 9)] = option(t, 9, ordinal(o, "Format", FORMATS, 0)?);
            r[pos(t, 10)] = Value::Guid(c.owner(o));
            // InherentPermissions and InherentEntitlements as declared, blank when not
            // (xmlport/lt-y-xmlport-props/YXmlPortProps.al:736-740: `X` for `X`).
            for name in ["InherentPermissions", "InherentEntitlements"] {
                if let Some(col) = t.fields.iter().position(|f| f.name == name) {
                    let v = prop(o, name).map(ident).unwrap_or_default();
                    if !matches!(v.as_str(), "" | "X") {
                        return Err(format!("xmlport '{}': {name} = {v} is not pinned (only X is)", o.name));
                    }
                    r[col] = Value::Text(v.into());
                }
            }
            Ok(())
        },
    )
}

/// Query Metadata: ID (1), Name (2), Caption (3), APIPublisher (4), APIGroup (5), APIVersion (6),
/// EntitySetName (7), EntityName (8), "App ID" (10). A query that is not an API query leaves the
/// API columns blank, but for APIVersion, which reads `beta` (TestQueryMetadataVirtualTable.al:107-115, query/W7QueryFilters.al). An API query that declares no
/// version is not pinned and is refused.
pub(super) fn query_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    entries(
        &c,
        t,
        |o| match (o.id, o.kind) {
            (Some(id), al_ir::ObjKind::Query) => Some(vec![(1, Value::Integer(id as i32))]),
            _ => None,
        },
        |o, r| {
            let api = prop(o, "QueryType").is_some_and(|v| ident(v).eq_ignore_ascii_case("API"));
            if api && prop(o, "APIVersion").is_none() {
                return Err(format!(
                    "query '{}': an API query that declares no APIVersion is not pinned",
                    o.name
                ));
            }
            r[pos(t, 2)] = Value::Text(truncate(&o.name, 30).into());
            r[pos(t, 3)] = Value::Text(truncate(&caption(o), 80).into());
            r[pos(t, 4)] = Value::Text(text(o, "APIPublisher", 40).into());
            r[pos(t, 5)] = Value::Text(text(o, "APIGroup", 40).into());
            // A query that declares no APIVersion reads `beta` (query/W7QueryFilters.al:
            // QueryMetadata_NormalQuery_ApiColumnsAreBlank; pages read the same, TestPageMetadataProperties.al:94-97).
            r[pos(t, 6)] = Value::Text(
                match prop(o, "APIVersion") {
                    None => "beta".to_string(),
                    Some(_) => text(o, "APIVersion", 250),
                }
                .into(),
            );
            r[pos(t, 7)] = Value::Text(text(o, "EntitySetName", 250).into());
            r[pos(t, 8)] = Value::Text(text(o, "EntityName", 250).into());
            r[pos(t, 10)] = Value::Guid(c.owner(o));
            Ok(())
        },
    )
}

// ---- Event Subscription -----------------------------------------------------------------------------

/// Event Subscription: "Subscriber Codeunit ID" (1), "Subscriber Function" (2), "Event Type" (3),
/// "Publisher Object Type" (4), "Publisher Object ID" (5), "Published Function" (6), Active (7).
/// One row per `[EventSubscriber]` routine of a compiled codeunit (record/TestEventSubscriptionVirtualTable.al).
///
/// A codeunit of a dependency that was not compiled may declare subscribers the program does not
/// hold, so each is one poisoned row with an empty function: a read that its key filters exclude is
/// answered, any other read is refused.
pub(super) fn event_subscription_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    for o in c.objects {
        let Some(id) = o.id else { continue };
        if o.is_shell_of(al_ir::ObjKind::Codeunit) {
            let mut k = blank(t);
            k[pos(t, 1)] = Value::Integer(id as i32);
            out.push(Entry::poisoned_prefix(
                k,
                1,
                format!(
                    "codeunit '{}' was not compiled, so the event subscribers it declares are not known",
                    o.name
                ),
            ));
            continue;
        }
        if !o.is_kind(al_ir::ObjKind::Codeunit) {
            continue;
        }
        // The head lists every routine with an event attribute, `EventSubscriber` among them, in routine order.
        for r in &o.events {
            let Some(a) = r
                .attributes
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
            else {
                continue;
            };
            let mut key = blank(t);
            key[pos(t, 1)] = Value::Integer(id as i32);
            key[pos(t, 2)] = Value::Text(truncate(&r.name, 250).into());
            let mut row = key.clone();
            match subscription_row(&c, o, &r.name, a, t, &mut row) {
                Ok(()) => out.push(Entry::ok(row)),
                // The columns worked out before the refusal are real: a filter on the publisher's
                // object type rules the row out without reading the part that is not pinned.
                Err((why, cols)) => {
                    out.push(Entry::poisoned(row, why).with_known_cols(cols.iter().map(|c| pos(t, *c)).collect()))
                }
            }
        }
    }
    out
}

fn subscription_row(
    c: &Catalog,
    o: &ObjectHead,
    routine: &str,
    a: &al_ir::Attribute,
    t: &TableDef,
    r: &mut al_data::Row,
) -> Result<(), (String, Vec<u32>)> {
    let what = || format!("subscriber '{}' of codeunit '{}'", routine, o.name);
    let manual = al_ir::ObjectFacts::of_properties(&o.properties).manual_subscriber;
    // "Subscriber Instance" reads Static-Automatic or Manual, "Error Information" is blank (record/
    // TestEventSubscriptionColumnsReportLayoutListContracts.al:98-107). "Number of Calls" and
    // "Active Manual Instances" are the session's, added when a row is read.
    r[pos(t, 12)] = Value::Text(if manual { "Manual" } else { "Static-Automatic" }.into());
    // "Originating App Name" is the name of the app that declares the subscriber (events/lt-event-subscription-trigger-rows/
    // EventSubscriptionTriggerRows.al:187-190 and the rows after it, `AL Language Coverage Tests`).
    r[pos(t, 11)] = Value::Text(
        truncate(
            &c.apps.get(o.app as usize).map(|x| x.name.clone()).unwrap_or_default(),
            250,
        )
        .into(),
    );
    let mut known = vec![11u32, 12u32];
    let args = crate::events::split_args(&a.text);
    if args.len() < 3 {
        return Err((
            format!("{}: the attribute has fewer than three arguments", what()),
            known,
        ));
    }
    let kind_text = crate::events::unquote(args[0].rsplit("::").next().unwrap_or("")).to_ascii_lowercase();
    let kind: al_ir::ObjKind = kind_text.parse().unwrap_or_default();
    let Some(object_type) = super::objects::object_type(kind) else {
        return Err((format!("{}: publisher kind `{kind_text}` is not listed", what()), known));
    };
    r[pos(t, 4)] = option(t, 4, object_type);
    known.push(4);
    let (target, _) = crate::events::publisher_name(args[1].rsplit("::").next().unwrap_or(""));
    let publisher_id = c
        .id_of(kind, &target)
        .map_err(|e| (format!("{}: publisher {e}", what()), known.clone()))?;
    r[pos(t, 5)] = Value::Integer(publisher_id as i32);
    known.push(5);
    let event = crate::events::unquote(&args[2]);
    // The publisher's own routine says which kind of event it is; a table's database triggers are
    // the platform's (event names `On{Before,After}{Insert,Modify,Delete,Rename,Validate}Event`).
    let publisher = c.objects.iter().find(|x| x.id == Some(publisher_id) && x.kind == kind);
    // (Only a routine with an event attribute can answer, and the head lists all of them.)
    let declared = publisher.and_then(|p| {
        p.events.iter().find_map(|pr| {
            if !pr.name.eq_ignore_ascii_case(&event) {
                return None;
            }
            pr.attributes
                .iter()
                .find_map(|pa| match pa.name.to_ascii_lowercase().as_str() {
                    "businessevent" => Some((pr.name.clone(), 1)),
                    "integrationevent" => Some((pr.name.clone(), 2)),
                    "internalevent" => Some((pr.name.clone(), 3)),
                    _ => None,
                })
        })
    });
    // A table trigger event bound to a field is published as `<event> - <field>`
    // (xmlport/TestPlatformMetadataDefaultsContracts.al:113-124, `OnBeforeValidateEvent` with `'Txt'`).
    // For another trigger event the element is not pinned.
    let element = args.get(3).map(|e| crate::events::unquote(e)).unwrap_or_default();
    let (published, event_type) = match declared {
        Some(d) => d,
        None if kind == al_ir::ObjKind::Table && is_trigger_event(&event) && element.is_empty() => (event.clone(), 0),
        None if kind == al_ir::ObjKind::Table
            && is_trigger_event(&event)
            && event.to_ascii_lowercase().ends_with("validateevent") =>
        {
            (format!("{event} - {element}"), 0)
        }
        None if kind == al_ir::ObjKind::Table && is_trigger_event(&event) => {
            return Err((
                format!(
                    "{}: the Published Function of `{event}` with the element `{element}` is not pinned",
                    what()
                ),
                known,
            ));
        }
        // A page action event's row has Event Type = Trigger and Active = Yes (events/lt-event-subscription-trigger-rows/
        // EventSubscriptionTriggerRows.al:222 for OnAfterActionEvent; events/lt-wd/WdActionEventRow.al:76-85 and :106-110 for
        // OnBeforeActionEvent, also as filters; corpus branch al-runtime-rs-staging-1030c, al-runner issue 6293). Its Published
        // Function is `<event> - <number>`, the number the id of the action's control: FNV-1a of the declaring object's id and the
        // action name (`pages::control_id`), the page extension's id for an action an extension adds, under the base page
        // (corpus master, corpus PR 886: events/lt-event-subscription-trigger-rows/ActionEventPublishedFunction.al:259-272, twelve
        // numbers; EventSubscriptionTriggerRows.al:222). An action the runtime cannot find on the page leaves that column refused.
        None if kind == al_ir::ObjKind::Page && is_action_event(&event) => {
            r[pos(t, 3)] = option(t, 3, 0);
            r[pos(t, 4)] = option(t, 4, object_type);
            r[pos(t, 5)] = Value::Integer(publisher_id as i32);
            r[pos(t, 6)] = match action_control_id(c, publisher_id, &element) {
                Some(id) => Value::Text(truncate(&format!("{event} - {id}"), 250).into()),
                None => super::unknown(
                    t,
                    6,
                    &format!("{} (the number in `{event} - <number>` of a page action event)", what()),
                ),
            };
            r[pos(t, 7)] = Value::Boolean(true);
            return Ok(());
        }
        None => return Err((format!("{}: the kind of event `{event}` is not known", what()), known)),
    };
    r[pos(t, 3)] = option(t, 3, event_type);
    r[pos(t, 4)] = option(t, 4, object_type);
    r[pos(t, 5)] = Value::Integer(publisher_id as i32);
    r[pos(t, 6)] = Value::Text(truncate(&published, 250).into());
    // A statically bound subscriber that resolved is active (TestEventSubscriptionVirtualTable.al:`_Row_DescribesTheSubscription`).
    r[pos(t, 7)] = Value::Boolean(true);
    Ok(())
}

/// The control id of the action `name` of page `page_id` (`pages::control_id` of the object that declares it): `None` when the
/// page is not compiled or does not hold exactly one action of that name.
fn action_control_id(c: &Catalog, page_id: u32, name: &str) -> Option<i64> {
    let index = c
        .objects
        .iter()
        .position(|h| h.kind == al_ir::ObjKind::Page && h.id == Some(page_id))?;
    let page = c.full[index].page.as_ref()?;
    let mut found = page
        .controls
        .iter()
        .filter(|ctl| ctl.kind == al_ir::ControlKind::Action && ctl.action_ref.is_none() && ctl.name.eq_ignore_ascii_case(name));
    let ctl = found.next()?;
    if found.next().is_some() {
        return None;
    }
    let declaring = match ctl.owner {
        None => page_id,
        Some(ext) => c.objects.get(ext)?.id?,
    };
    Some(crate::pages::control_id(declaring, &ctl.name))
}

fn is_action_event(event: &str) -> bool {
    event.eq_ignore_ascii_case("OnBeforeActionEvent") || event.eq_ignore_ascii_case("OnAfterActionEvent")
}

fn is_trigger_event(event: &str) -> bool {
    let e = event.to_ascii_lowercase();
    ["onbefore", "onafter"].iter().any(|p| {
        e.strip_prefix(p).is_some_and(|rest| {
            ["insert", "modify", "delete", "rename", "validate"]
                .iter()
                .any(|k| rest.strip_prefix(k) == Some("event"))
        })
    })
}

// ---- Page Action ------------------------------------------------------------------------------------

/// Page Action: "Page ID" (1), "Action Index" (2), "Action ID" (3), Indentation (4), "Parent Action ID" (5),
/// "Action Type" (6), Name (8), RunObjectID (16). One row per `area(...)` of `actions` (an action
/// container, type 0), then one per action (1) and action group (3) below it, depth first
/// (record/TestPageActionVirtualTable.al).
///
/// The corpus pins that a group's parent is a row of the table at indentation 0, that its
/// actions sit one level deeper with the group as parent, and that the three types differ. It does
/// not pin the numbers: the index and the id both count from 1 in this order, and a container has
/// parent 0 (docs/probes-needed.md).
pub(super) fn page_action_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    for (i, h) in c.objects.iter().enumerate() {
        let (Some(id), al_ir::ObjKind::Page) = (h.id, h.kind) else {
            continue;
        };
        // The actions are in the page itself: only the pages are made.
        let o = &c.full[i];
        let mut key = blank(t);
        key[pos(t, 1)] = Value::Integer(id as i32);
        let Some(page) = &o.page else {
            out.push(Entry::poisoned_prefix(
                key,
                1,
                format!("page '{}' was not compiled, so its actions are not known", o.name),
            ));
            continue;
        };
        match page_actions(&c, o, page, t, &key) {
            Ok(rows) => out.extend(rows.into_iter().map(Entry::ok)),
            Err(why) => out.push(Entry::poisoned_prefix(key, 1, why)),
        }
    }
    out
}

fn page_actions(
    c: &Catalog,
    o: &Object,
    page: &al_ir::Page,
    t: &TableDef,
    key: &al_data::Row,
) -> Result<Vec<al_data::Row>, String> {
    use al_ir::ControlKind as K;
    let mut rows = Vec::new();
    let mut next = 1;
    for area in &page.action_areas {
        if area.name.is_empty() {
            return Err(format!(
                "page '{}': actions written outside an area are not pinned",
                o.name
            ));
        }
        let container = next;
        let mut r = key.clone();
        r[pos(t, 2)] = Value::Integer(next);
        r[pos(t, 3)] = Value::Integer(next);
        r[pos(t, 4)] = Value::Integer(0);
        r[pos(t, 5)] = Value::Integer(0);
        r[pos(t, 6)] = option(t, 6, 0);
        r[pos(t, 8)] = Value::Text(truncate(&area.name, 120).into());
        rows.push(r);
        next += 1;
        // Depth first below the container.
        let mut stack: Vec<(usize, i32, i32)> = area.roots.iter().rev().map(|&i| (i, 1, container)).collect();
        while let Some((ci, depth, parent)) = stack.pop() {
            let ctl = &page.controls[ci];
            let kind = match ctl.kind {
                K::Action if ctl.action_ref.is_none() => 1,
                K::ActionGroup => 3,
                _ => {
                    return Err(format!(
                        "page '{}': the action '{}' is a {:?}{}, whose row is not pinned",
                        o.name,
                        ctl.name,
                        ctl.kind,
                        if ctl.action_ref.is_some() { " reference" } else { "" }
                    ));
                }
            };
            let mut r = key.clone();
            r[pos(t, 2)] = Value::Integer(next);
            r[pos(t, 3)] = Value::Integer(next);
            r[pos(t, 4)] = Value::Integer(depth);
            r[pos(t, 5)] = Value::Integer(parent);
            r[pos(t, 6)] = option(t, 6, kind);
            r[pos(t, 8)] = Value::Text(truncate(&ctl.name, 120).into());
            let run_id = match &ctl.run_object {
                None => 0,
                Some(ro) => match ro.object.and_then(|i| c.objects.get(i)).and_then(|x| x.id) {
                    Some(id) => id as i32,
                    None => {
                        return Err(format!(
                            "page '{}': action '{}' runs an object that is not compiled",
                            o.name, ctl.name
                        ));
                    }
                },
            };
            r[pos(t, 16)] = Value::Integer(run_id);
            rows.push(r);
            let me = next;
            next += 1;
            let kids: Vec<usize> = page
                .controls
                .iter()
                .enumerate()
                .filter(|(_, k)| k.parent == Some(ci))
                .map(|(i, _)| i)
                .collect();
            for k in kids.into_iter().rev() {
                stack.push((k, depth + 1, me));
            }
        }
    }
    Ok(rows)
}

// ---- Table Relations Metadata -----------------------------------------------------------------------

/// Table Relations Metadata: "Table ID" (1), "Field No." (2), "Relation No." (3), "Condition No." (4),
/// "Related Table ID" (5), "Related Field No." (6), "Condition Field No." (8). One row per arm of a
/// field's `TableRelation`, or per condition of an arm that has some; `where(...)` filters are not
/// conditions (record/TestTableRelationsMetadataVirtualTable.al). The arms number from 1 in
/// declaration order; "Condition No." counts a row's condition from 1 and is 0 for an arm with none,
/// and a row with no condition reports "Condition Field No." 0 (not pinned: docs/probes-needed.md).
pub(super) fn table_relations_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    for o in c.objects {
        let (Some(id), al_ir::ObjKind::Table) = (o.id, o.kind) else {
            continue;
        };
        let mut key = blank(t);
        key[pos(t, 1)] = Value::Integer(id as i32);
        let Some(table) = &o.table else {
            out.push(Entry::poisoned_prefix(
                key,
                1,
                format!(
                    "table '{}' was not compiled, so the relations of its fields are not known",
                    o.name
                ),
            ));
            continue;
        };
        for f in &table.fields {
            // SystemCreatedBy and SystemModifiedBy relate to the User table (field 1), tested and not
            // validated (record/TestTableRelationsSequenceMetadataContracts.al:66-67, 83-84).
            if matches!(f.id, 2_000_000_002 | 2_000_000_004) {
                let mut r = key.clone();
                r[pos(t, 2)] = Value::Integer(f.id as i32);
                r[pos(t, 3)] = Value::Integer(1);
                r[pos(t, 4)] = Value::Integer(1);
                r[pos(t, 5)] = Value::Integer(2_000_000_120);
                r[pos(t, 6)] = Value::Integer(1);
                r[pos(t, 10)] = Value::Boolean(true);
                out.push(Entry::ok(r));
                continue;
            }
            if !f.relation {
                continue;
            }
            let mut fkey = key.clone();
            fkey[pos(t, 2)] = Value::Integer(f.id as i32);
            match field_relations(&c, o, table, f, t, &fkey) {
                Ok(rows) => out.extend(rows.into_iter().map(Entry::ok)),
                Err(why) => out.push(Entry::poisoned_prefix(fkey, 2, why)),
            }
        }
    }
    out
}

/// One arm of a relation as the table lists it: the related table and field, and the ids of the
/// fields of the own table its conditions name.
struct Arm {
    related_table: u32,
    related_field: u32,
    conditions: Vec<u32>,
    /// The Condition Value text of each condition (the ordinal a `const(...)` names, the ordinals of a
    /// `filter(A | B)` joined by `|`), when it is known.
    values: Vec<Option<String>>,
    /// Each condition is a `filter(...)` (Condition Type FILTER) and not a `const(...)`.
    filters: Vec<bool>,
}

/// The first primary key field of a compiled table.
fn first_key_field(related: &ObjectHead, what: &str) -> Result<u32, String> {
    let rt = related
        .table
        .as_ref()
        .ok_or_else(|| format!("{what}: related table '{}' was not compiled", related.name))?;
    rt.keys
        .first()
        .and_then(|k| k.first())
        .and_then(|p| rt.fields.get(*p))
        .map(|f| f.id)
        .ok_or_else(|| format!("{what}: related table '{}' has no key", related.name))
}

fn arms_of(c: &Catalog, table: &al_ir::Table, f: &al_ir::Field, what: &str) -> Result<Vec<Arm>, String> {
    let mut out = Vec::new();
    if !f.relation_arms.is_empty() {
        for arm in &f.relation_arms {
            let related = arm
                .table
                .and_then(|i| c.objects.get(i))
                .ok_or_else(|| format!("{what}: related table '{}' is not in the program", arm.table_name))?;
            let related_table = related.id.ok_or_else(|| format!("{what}: related table has no id"))?;
            let related_field = match arm.field {
                Some(p) => related
                    .table
                    .as_ref()
                    .and_then(|rt| rt.fields.get(p))
                    .map(|rf| rf.id)
                    .ok_or_else(|| format!("{what}: field of '{}' not compiled", related.name))?,
                None => first_key_field(related, what)?,
            };
            let conditions = arm
                .conditions
                .iter()
                .map(|cond| {
                    table
                        .fields
                        .get(cond.field)
                        .map(|x| x.id)
                        .ok_or_else(|| format!("{what}: condition field not found"))
                })
                .collect::<Result<_, _>>()?;
            let values = arm
                .conditions
                .iter()
                .map(|cond| match &cond.value {
                    al_ir::Const::Enum(_, v) => Some(v.to_string()),
                    al_ir::Const::Option(i, _) => Some(i.to_string()),
                    _ => None,
                })
                .collect();
            out.push(Arm {
                related_table,
                related_field,
                conditions,
                values,
                filters: Vec::new(),
            });
        }
        return Ok(out);
    }
    // The runtime does not understand this relation (a `filter(...)` condition, for one); the
    // compile kept the tables and fields it names.
    if f.relation_meta.is_empty() {
        return Err(format!("{what}: the shape of its TableRelation is not understood"));
    }
    for arm in &f.relation_meta {
        let related = &c.objects[arm.table];
        let related_table = related.id.ok_or_else(|| format!("{what}: related table has no id"))?;
        let related_field = match arm.field {
            Some(p) => related
                .table
                .as_ref()
                .and_then(|rt| rt.fields.get(p))
                .map(|rf| rf.id)
                .ok_or_else(|| format!("{what}: field of '{}' not compiled", related.name))?,
            None => first_key_field(related, what)?,
        };
        let conditions = arm
            .conditions
            .iter()
            .map(|&p| {
                table
                    .fields
                    .get(p)
                    .map(|x| x.id)
                    .ok_or_else(|| format!("{what}: condition field not found"))
            })
            .collect::<Result<_, _>>()?;
        out.push(Arm {
            related_table,
            related_field,
            conditions,
            values: arm.values.clone(),
            filters: arm.filters.clone(),
        });
    }
    Ok(out)
}

fn field_relations(
    c: &Catalog,
    o: &ObjectHead,
    table: &al_ir::Table,
    f: &al_ir::Field,
    t: &TableDef,
    fkey: &al_data::Row,
) -> Result<Vec<al_data::Row>, String> {
    let what = format!("field '{}' of table '{}'", f.name, o.name);
    let mut rows = Vec::new();
    // ValidateTableRelation = false is declared; a relation validates otherwise. Test Table
    // Relation is No for every declared relation (record/TestTableRelationsSequenceMetadataContracts.al:63-71).
    let validate = al_ir::FieldFacts::of(f).validate_relation;
    for (n, arm) in arms_of(c, table, f, &what)?.iter().enumerate() {
        let mut row = fkey.clone();
        row[pos(t, 3)] = Value::Integer(n as i32 + 1);
        row[pos(t, 5)] = Value::Integer(arm.related_table as i32);
        row[pos(t, 6)] = Value::Integer(arm.related_field as i32);
        row[pos(t, 11)] = Value::Boolean(validate);
        // An arm with no condition is one row with Condition No. 1, Condition Field No. 0, type CONST and
        // an empty value (ibid.).
        if arm.conditions.is_empty() {
            row[pos(t, 4)] = Value::Integer(1);
            row[pos(t, 8)] = Value::Integer(0);
            rows.push(row.clone());
        }
        for (k, own) in arm.conditions.iter().enumerate() {
            let mut r = row.clone();
            r[pos(t, 4)] = Value::Integer(k as i32 + 1);
            r[pos(t, 8)] = Value::Integer(*own as i32);
            let filter = arm.filters.get(k).copied().unwrap_or(false);
            // A filter whose members are not all named options reads a blank value: its text form is not
            // pinned (docs/probes-needed.md, "Probe fixes (corpus 345627d)").
            if let Some(Some(v)) = arm.values.get(k) {
                r[pos(t, 9)] = Value::Text(v.clone().into());
            }
            if filter {
                // Condition Type: CONST is the first member, FILTER the second (session/PWPlatformTables.al:109-123).
                r[pos(t, 7)] = option(t, 7, 1);
            }
            rows.push(r);
        }
    }
    Ok(rows)
}

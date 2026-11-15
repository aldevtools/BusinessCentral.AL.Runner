//! The Event Subscription row of a subscriber to an event that an extension of the publisher object declares
//! (al-runner issue 8060). The corpus pins that such a subscriber, bound to the base object, fires
//! (events/extension-events/TestExtensionDeclaredEvents.al:92), and its whole row in the cells of [`ROW_CELLS`]
//! (events/extension-events/ExtensionEventSubscriptionRows.al:178-281). In the two cells only Microsoft's Tests-Misc codeunit
//! 134299 measures ([`ACTIVE_CELLS`]), Active and Error Information rest on assumption `extension-event-subscriber-active`.
//! Every other cell is refused.

use super::objects::prop;
use al_ir::{ObjKind, ObjectFacts, ObjectHead, PageType};

/// The base object as the measured cells hold it, each fact by its one measured value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Base {
    /// A table that states no TableType (ExtensionEventSubscriptionRows.al:14-24; System Application's "Retention Policy
    /// Setup" states none either).
    Table,
    /// A page with `PageType = Card` (:26-29).
    CardPage,
    /// A page with `PageType = ListPart` (System Application's "Retention Policy Setup Lines", an [`ACTIVE_CELLS`] cell).
    ListPartPage,
    /// A page with `PageType = List` (Base Application's "Error Messages", an [`ACTIVE_CELLS`] cell).
    ListPage,
    /// A report with `ProcessingOnly = true` (:41-44).
    ProcessingOnlyReport,
}

impl Base {
    fn of(publisher: &ObjectHead) -> Option<Base> {
        let facts = ObjectFacts::of_properties(&publisher.properties);
        match publisher.kind {
            ObjKind::Table if facts.table_type.is_none() => Some(Base::Table),
            ObjKind::Page if facts.page_type == Some(PageType::Card) => Some(Base::CardPage),
            ObjKind::Page if facts.page_type == Some(PageType::Other("listpart".into())) => Some(Base::ListPartPage),
            ObjKind::Page if facts.page_type == Some(PageType::List) => Some(Base::ListPage),
            ObjKind::Report if prop(publisher, "ProcessingOnly").is_some_and(|v| v.eq_ignore_ascii_case("true")) => {
                Some(Base::ProcessingOnlyReport)
            }
            _ => None,
        }
    }
}

/// Where the base object sits against the extension that declares the event, whose app is the subscriber's app in every
/// measured cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Placement {
    /// The base object, the extension and the subscriber codeunit are in one app.
    OneApp,
    /// The extension and the subscriber codeunit are in one app, the base object in another.
    BaseInAnotherApp,
}

/// The event attribute of the extension's routine, with its arguments as written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EventAttr {
    /// `[IntegrationEvent(false, false)]`.
    Integration,
    /// `[IntegrationEvent(true, false)]`.
    IntegrationWithSender,
    /// `[BusinessEvent(false)]`.
    Business,
}

/// A measured cell: the base object (the extension kind is its pair), the event attribute, the subscriber's
/// SkipOnMissingLicense and SkipOnMissingPermission, whether its codeunit is EventSubscriberInstance = Manual, and the
/// placement. In every cell exactly one extension of the publisher declares the event, as a `local` procedure, under the name
/// the subscriber writes; the subscriber's element is '' and its codeunit states neither Subtype nor SingleInstance
/// (ExtensionEventSubscriptionRows.al:52-150; the Base Application objects of [`ACTIVE_CELLS`]).
type Cell = (Base, EventAttr, bool, bool, bool, Placement);

/// The cells of events/extension-events/ExtensionEventSubscriptionRows.al (identical on BC 27.5, 28.5 and 29.0): Event Type Integration or Business as declared, Publisher Object Type and ID of the base object,
/// Published Function the event's name, Active Yes, Error Information blank.
const ROW_CELLS: &[Cell] = &[
    // :178-188
    (
        Base::Table,
        EventAttr::Integration,
        false,
        false,
        false,
        Placement::OneApp,
    ),
    // :190-198
    (
        Base::Table,
        EventAttr::IntegrationWithSender,
        false,
        false,
        false,
        Placement::OneApp,
    ),
    // :200-208
    (Base::Table, EventAttr::Business, false, false, false, Placement::OneApp),
    // :210-218
    (
        Base::Table,
        EventAttr::Integration,
        true,
        true,
        false,
        Placement::OneApp,
    ),
    // :220-230
    (
        Base::CardPage,
        EventAttr::Integration,
        false,
        false,
        false,
        Placement::OneApp,
    ),
    // :232-242
    (
        Base::ProcessingOnlyReport,
        EventAttr::Integration,
        false,
        false,
        false,
        Placement::OneApp,
    ),
    // :244-252
    (
        Base::Table,
        EventAttr::Integration,
        false,
        false,
        true,
        Placement::OneApp,
    ),
    // :254-264 (the reporter's shape: a table extension of System Application's "Retention Policy Setup")
    (
        Base::Table,
        EventAttr::Integration,
        false,
        false,
        false,
        Placement::BaseInAnotherApp,
    ),
];

/// The cells only Microsoft's tests measure, Active Yes and a blank Error Information and nothing else (Tests-Misc codeunit
/// 134299 "Test Partner Integration Event", TestPartnerIntegrationEvent.Codeunit.al:157-191, TestAllSubscriptionsAreActive and
/// TestSubscriptionTableHasNoErrors; the facts are alike in the BC 27.5, 28.5 and 29.0 sources): Base Application's "Serv.
/// Integration Mgt".SetIsDocumentArchiveTable_OnAfterSetIsDocumentArchiveTable on System Application's "Retention Policy Setup
/// Lines", pageextension 3997 "Reten. Pol. Setup Lines Ext."; Base Application's "Check Dimensions".OnOpenRelatedRecord on
/// "Error Messages", pageextension 705 "Error Messages Extension", all three in Base Application. Assumption
/// `extension-event-subscriber-active`.
const ACTIVE_CELLS: &[Cell] = &[
    (
        Base::ListPartPage,
        EventAttr::Integration,
        true,
        true,
        false,
        Placement::BaseInAnotherApp,
    ),
    (
        Base::ListPage,
        EventAttr::Integration,
        false,
        false,
        false,
        Placement::OneApp,
    ),
];

/// What a measured cell knows of the row.
pub(super) enum Measured<'a> {
    /// The whole row ([`ROW_CELLS`]): the routine's name as declared (the Published Function) and the Event Type's option
    /// value (1 Business, 2 Integration, as `metadata::subscription_row` numbers a declared event).
    Row { published: String, event_type: i32 },
    /// Active and Error Information only ([`ACTIVE_CELLS`]).
    ActiveOnly { extension: &'a ObjectHead },
}

/// What a measured cell knows of the row of a subscription to `event` of `publisher`; `None` for any other shape. `args` are
/// the subscriber attribute's arguments, `manual` whether its codeunit is EventSubscriberInstance = Manual.
pub(super) fn measured<'a>(
    objects: &'a [ObjectHead],
    subscriber: &ObjectHead,
    manual: bool,
    publisher: &'a ObjectHead,
    event: &str,
    args: &[String],
) -> Option<Measured<'a>> {
    let flag = |s: &str| match crate::events::unquote(s).to_ascii_lowercase().as_str() {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    };
    (args.len() == 6 && crate::events::unquote(&args[3]).is_empty()).then_some(())?;
    let (license, permission) = (flag(&args[4])?, flag(&args[5])?);
    let base = Base::of(publisher)?;
    let plain_codeunit = (prop(subscriber, "Subtype"), prop(subscriber, "SingleInstance")) == (None, None);
    plain_codeunit.then_some(())?;
    // Found without regard to case, so that a declaration in a second extension counts; each measured subscriber writes
    // the name as declared (the equality below).
    let mut declaring = extensions_of(objects, publisher)?.filter_map(|ext| {
        let routine = ext.events.iter().find(|r| r.name.eq_ignore_ascii_case(event))?;
        let attribute = routine.attributes.iter().find(|a| event_attribute(&a.name))?;
        Some((ext, routine, attribute))
    });
    let (ext, routine, attribute) = declaring.next()?;
    let measured = declaring.next().is_none() && subscriber.app == ext.app && routine.name == event && routine.local;
    measured.then_some(())?;
    let published = routine.name.clone();
    let attr_args: Vec<Option<bool>> = crate::events::split_args(&attribute.text)
        .iter()
        .map(|a| flag(a))
        .collect();
    let attr = match (attribute.name.to_ascii_lowercase().as_str(), attr_args.as_slice()) {
        ("integrationevent", [Some(false), Some(false)]) => EventAttr::Integration,
        ("integrationevent", [Some(true), Some(false)]) => EventAttr::IntegrationWithSender,
        ("businessevent", [Some(false)]) => EventAttr::Business,
        _ => return None,
    };
    let placement = if publisher.app == ext.app {
        Placement::OneApp
    } else {
        Placement::BaseInAnotherApp
    };
    let cell = (base, attr, license, permission, manual, placement);
    if ROW_CELLS.contains(&cell) {
        let event_type = if attr == EventAttr::Business { 1 } else { 2 };
        return Some(Measured::Row { published, event_type });
    }
    ACTIVE_CELLS
        .contains(&cell)
        .then_some(Measured::ActiveOnly { extension: ext })
}

/// The extensions of `publisher` of its paired kind: table extensions of a table, page extensions of a page, report
/// extensions of a report; `None` for another kind.
fn extensions_of<'a>(
    objects: &'a [ObjectHead],
    publisher: &'a ObjectHead,
) -> Option<Box<dyn Iterator<Item = &'a ObjectHead> + 'a>> {
    let listed = |list: &'a [usize], kind: ObjKind| -> Box<dyn Iterator<Item = &'a ObjectHead> + 'a> {
        Box::new(
            list.iter()
                .filter_map(|&e| objects.get(e))
                .filter(move |e| e.kind == kind),
        )
    };
    match publisher.kind {
        ObjKind::Table => Some(listed(&publisher.table.as_ref()?.extensions, ObjKind::TableExtension)),
        ObjKind::Page => Some(listed(&publisher.page_extensions, ObjKind::PageExtension)),
        ObjKind::Report => {
            let index = objects.iter().position(|o| std::ptr::eq(o, publisher))?;
            Some(Box::new(objects.iter().filter(move |e| {
                e.kind == ObjKind::ReportExtension && e.report_ext_base == Some(index)
            })))
        }
        _ => None,
    }
}

fn event_attribute(name: &str) -> bool {
    ["IntegrationEvent", "BusinessEvent", "InternalEvent"]
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
}

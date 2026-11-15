//! The Event Subscription row of a subscriber to an event that an extension of the publisher object declares
//! (al-runner issue 8060). The corpus pins that such a subscriber, bound to the base object, fires
//! (events/extension-events/TestExtensionDeclaredEvents.al:92); it does not pin its row. Microsoft's Tests-Misc codeunit
//! 134299 TestAllSubscriptionsAreActive and TestSubscriptionTableHasNoErrors read every row of a database whose Base
//! Application holds three such subscribers ([`CELLS`]), and pass on real BC: their rows have Active Yes and a blank Error
//! Information, or there are none. Assumption `extension-event-subscriber-active`, in those cells only.

use al_ir::{ObjKind, ObjectHead};

/// Where the base object sits against the extension that declares the event, whose app is the subscriber's app in every
/// measured cell.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    /// The base object, the extension and the subscriber codeunit are in one app.
    OneApp,
    /// The extension and the subscriber codeunit are in one app, the base object in another.
    BaseInAnotherApp,
}

/// The measured cells: the publisher kind (the extension kind is its pair, TableExtension or PageExtension), the subscriber's
/// SkipOnMissingLicense and SkipOnMissingPermission, and the placement. In every cell the event is
/// `[IntegrationEvent(false, false)]` that exactly one extension of the publisher declares, the subscriber's element is '' and
/// its codeunit is not EventSubscriberInstance = Manual. Base Application (BC 29.0 source), whose rows the Microsoft tests read:
/// - Table, false, false, one app: "Check Dimensions".OnErrorMessageDrillDown on "Error Message", tableextension 705
///   "Error Message Extension".OnDrillDownSource;
/// - Page, false, false, one app: "Check Dimensions".OnOpenRelatedRecord on "Error Messages", pageextension 705
///   "Error Messages Extension".OnOpenRelatedRecord;
/// - Page, true, true, base in another app: "Serv. Integration Mgt.".SetIsDocumentArchiveTable_OnAfterSetIsDocumentArchiveTable on
///   System Application's "Retention Policy Setup Lines", pageextension 3997 "Reten. Pol. Setup Lines Ext.".
const CELLS: &[(ObjKind, bool, bool, Placement)] = &[
    (ObjKind::Table, false, false, Placement::OneApp),
    (ObjKind::Page, false, false, Placement::OneApp),
    (ObjKind::Page, true, true, Placement::BaseInAnotherApp),
];

/// The extension that declares `event` for `publisher`, when the subscription is in a measured cell ([`CELLS`]); `None` for
/// any other shape. `args` are the subscriber attribute's arguments.
pub(super) fn measured_extension<'a>(
    objects: &'a [ObjectHead],
    subscriber: &ObjectHead,
    manual: bool,
    publisher: &ObjectHead,
    event: &str,
    args: &[String],
) -> Option<&'a ObjectHead> {
    let (extensions, extension_kind): (&[usize], ObjKind) = match publisher.kind {
        ObjKind::Table => (publisher.table.as_ref()?.extensions.as_slice(), ObjKind::TableExtension),
        ObjKind::Page => (publisher.page_extensions.as_slice(), ObjKind::PageExtension),
        _ => return None,
    };
    let flag = |s: &str| match crate::events::unquote(s).to_ascii_lowercase().as_str() {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    };
    let blank_element = args.len() == 6 && crate::events::unquote(&args[3]).is_empty();
    if !(blank_element && !manual) {
        return None;
    }
    let (license, permission) = (flag(&args[4])?, flag(&args[5])?);
    let mut declaring = extensions.iter().filter_map(|&e| {
        let ext = objects.get(e)?;
        let attribute = ext
            .events
            .iter()
            .filter(|r| r.name.eq_ignore_ascii_case(event))
            .find_map(|r| r.attributes.iter().find(|a| event_attribute(&a.name)))?;
        Some((ext, attribute))
    });
    let (ext, attribute) = declaring.next()?;
    let one_declaring = declaring.next().is_none();
    let event_args = crate::events::split_args(&attribute.text);
    let integration = attribute.name.eq_ignore_ascii_case("IntegrationEvent")
        && event_args.len() == 2
        && event_args.iter().all(|a| flag(a) == Some(false));
    let placement = if publisher.app == ext.app {
        Placement::OneApp
    } else {
        Placement::BaseInAnotherApp
    };
    let measured = one_declaring
        && ext.kind == extension_kind
        && integration
        && subscriber.app == ext.app
        && CELLS.contains(&(publisher.kind, license, permission, placement));
    measured.then_some(ext)
}

fn event_attribute(name: &str) -> bool {
    ["IntegrationEvent", "BusinessEvent", "InternalEvent"]
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
}

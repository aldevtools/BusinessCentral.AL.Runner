//! Typed facts read from the properties of objects and fields (ADR-032 decision 5).
//!
//! The VM used to scan `Object::properties` and `Field::properties` as text on its hot paths (`TableType`,
//! `PageType`, `EventSubscriberInstance`, `SourceTableView`, `FieldClass`, `ValidateTableRelation`). They are worked
//! out once, when the engine is built, from the same text with the same comparison (trimmed, case-insensitive), so
//! nothing changes for AL code. A value the list does not know is kept (`Other`) and behaves as the text did.

use crate::{Field, Object};

/// `TableType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableType {
    Normal,
    Temporary,
    Crm,
    /// Any other value, lower case.
    Other(String),
}

/// `PageType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageType {
    Card,
    List,
    Document,
    Worksheet,
    ListPlus,
    ConfirmationDialog,
    StandardDialog,
    PromptDialog,
    NavigatePage,
    /// Any other value, lower case.
    Other(String),
}

impl TableType {
    /// The lower-case text, as the property reads.
    pub fn lower(&self) -> &str {
        match self {
            TableType::Normal => "normal",
            TableType::Temporary => "temporary",
            TableType::Crm => "crm",
            TableType::Other(o) => o,
        }
    }
}

impl PageType {
    pub fn from_text(text: &str) -> PageType {
        match text.trim().to_ascii_lowercase().as_str() {
            "card" => PageType::Card,
            "list" => PageType::List,
            "document" => PageType::Document,
            "worksheet" => PageType::Worksheet,
            "listplus" => PageType::ListPlus,
            "confirmationdialog" => PageType::ConfirmationDialog,
            "standarddialog" => PageType::StandardDialog,
            "promptdialog" => PageType::PromptDialog,
            "navigatepage" => PageType::NavigatePage,
            other => PageType::Other(other.to_string()),
        }
    }

    /// The lower-case text, as the property reads.
    pub fn lower(&self) -> &str {
        match self {
            PageType::Card => "card",
            PageType::List => "list",
            PageType::Document => "document",
            PageType::Worksheet => "worksheet",
            PageType::ListPlus => "listplus",
            PageType::ConfirmationDialog => "confirmationdialog",
            PageType::StandardDialog => "standarddialog",
            PageType::PromptDialog => "promptdialog",
            PageType::NavigatePage => "navigatepage",
            PageType::Other(o) => o,
        }
    }

    /// One of the five types a test page can be opened on (`Card`, `List`, `Document`, `Worksheet`, `ListPlus`).
    pub fn is_plain(&self) -> bool {
        matches!(
            self,
            PageType::Card | PageType::List | PageType::Document | PageType::Worksheet | PageType::ListPlus
        )
    }
}

/// What the VM asks of an object's properties.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObjectFacts {
    /// `TableType`, if the object states it.
    pub table_type: Option<TableType>,
    /// `PageType`, if the object states it.
    pub page_type: Option<PageType>,
    /// `EventSubscriberInstance = Manual` (a manual subscriber codeunit).
    pub manual_subscriber: bool,
    /// `SingleInstance = true` (a codeunit with one instance per session).
    pub single_instance: bool,
    /// The object states `SourceTableView`.
    pub has_source_table_view: bool,
    /// `ObsoleteState = Moved`: the object moved to another app and is not in the runtime app.
    pub moved: bool,
}

fn value<'a>(properties: &'a [(String, String)], name: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

impl ObjectFacts {
    pub fn of(o: &Object) -> ObjectFacts {
        ObjectFacts::of_properties(&o.properties)
    }

    pub fn of_properties(properties: &[(String, String)]) -> ObjectFacts {
        ObjectFacts {
            table_type: value(properties, "TableType").map(|v| match v.to_ascii_lowercase().as_str() {
                "normal" => TableType::Normal,
                "temporary" => TableType::Temporary,
                "crm" => TableType::Crm,
                other => TableType::Other(other.to_string()),
            }),
            page_type: value(properties, "PageType").map(PageType::from_text),
            manual_subscriber: properties.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("EventSubscriberInstance") && v.trim().eq_ignore_ascii_case("Manual")
            }),
            single_instance: properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true")),
            has_source_table_view: properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("SourceTableView")),
            moved: value(properties, "ObsoleteState").is_some_and(|v| v.eq_ignore_ascii_case("Moved")),
        }
    }

    /// The lower-case `PageType`, or `""` when the object states none.
    pub fn page_type_lower(&self) -> &str {
        self.page_type.as_ref().map_or("", |p| p.lower())
    }

    /// `TableType = Temporary`.
    pub fn temporary(&self) -> bool {
        self.table_type == Some(TableType::Temporary)
    }

    /// `TableType = CRM`.
    pub fn crm(&self) -> bool {
        self.table_type == Some(TableType::Crm)
    }
}

/// `FieldClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldClass {
    #[default]
    Normal,
    FlowField,
    FlowFilter,
}

/// What the VM asks of a field's properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldFacts {
    pub class: FieldClass,
    /// `ValidateTableRelation` is not `false`.
    pub validate_relation: bool,
}

impl FieldFacts {
    pub fn of(f: &Field) -> FieldFacts {
        let class = if f
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("FieldClass") && v.trim().eq_ignore_ascii_case("FlowField"))
        {
            FieldClass::FlowField
        } else if f
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("FieldClass") && v.trim().eq_ignore_ascii_case("FlowFilter"))
        {
            FieldClass::FlowFilter
        } else {
            FieldClass::Normal
        };
        FieldFacts {
            class,
            validate_relation: !f.properties.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("ValidateTableRelation") && v.trim().eq_ignore_ascii_case("false")
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(p: &[(&str, &str)]) -> Vec<(String, String)> {
        p.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn object_facts_read_the_properties_as_the_text_scans_did() {
        let f = ObjectFacts::of_properties(&props(&[
            ("tabletype", " Temporary "),
            ("PAGETYPE", "ListPlus"),
            ("EventSubscriberInstance", "manual"),
            ("sourcetableview", "sorting(a)"),
        ]));
        assert!(f.temporary() && !f.crm());
        assert_eq!(f.page_type, Some(PageType::ListPlus));
        assert_eq!(f.page_type_lower(), "listplus");
        assert!(f.manual_subscriber && f.has_source_table_view);
        let none = ObjectFacts::of_properties(&[]);
        assert_eq!((none.table_type.clone(), none.page_type_lower()), (None, ""));
        assert!(!none.manual_subscriber);
        assert_eq!(
            ObjectFacts::of_properties(&props(&[("TableType", "ExternalSQL")])).table_type,
            Some(TableType::Other("externalsql".into()))
        );
        assert!(PageType::from_text("Card").is_plain() && !PageType::from_text("StandardDialog").is_plain());
    }
}

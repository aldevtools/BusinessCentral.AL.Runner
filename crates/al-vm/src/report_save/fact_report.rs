//! The facts of the report a save renders: the default layout, how it is declared, the data items, the triggers and the
//! report extensions.

use super::Schema;
use super::facts::{DataItems, Decl, Declares, Extensions, Kind, Layout, LayoutFile, Triggers};

/// What the report declares, as the cells tell it apart.
pub(super) struct ReportFacts {
    pub kind: Kind,
    pub layout: Layout,
    pub decl: Decl,
    pub declares: Declares,
    pub file: LayoutFile,
    pub processing_only: bool,
    pub data_items: DataItems,
    pub extensions: Extensions,
    pub triggers: Triggers,
}

pub(super) fn report_facts(program: &al_ir::Program, schema: &Schema) -> ReportFacts {
    let report = schema.report();
    let kind = match report.default_layout_kind.as_str() {
        "rdlc" => Kind::Rdlc,
        "word" => Kind::Word,
        "excel" => Kind::Excel,
        _ => Kind::Other,
    };
    let data_items = match report.items.as_slice() {
        [_] => DataItems::One,
        [top, inner] if top.parent.is_none() && inner.parent == Some(0) => DataItems::ParentAndChild,
        [a, b] if a.parent.is_none() && b.parent.is_none() => DataItems::TwoTopLevel,
        [a, b, c] if a.parent.is_none() && b.parent.is_none() && c.parent.is_none() => DataItems::ThreeTopLevel,
        _ => DataItems::Other,
    };
    let extensions = match report.extensions.as_slice() {
        [] => Extensions::None,
        [e] if adds_columns_only(program, *e) => Extensions::OneAddingColumns,
        _ => Extensions::Other,
    };
    let triggers = if report.on_post.is_some() {
        Triggers::OnPost
    } else if report.on_init.is_some() || report.on_pre.is_some() || report.on_pre_rendering.is_some() {
        Triggers::WithoutOnPost
    } else {
        Triggers::None
    };
    ReportFacts {
        kind,
        layout: if report.layout == al_ir::ReportLayout::Rdlc {
            Layout::Rdlc
        } else {
            Layout::NotRdlc
        },
        decl: if report.default_rendering_layout.is_some() {
            Decl::Rendering
        } else {
            Decl::Legacy
        },
        // reports/html-email-body/RxdDeclFixtures.al:31, report 80040.
        declares: if report.rendering_layouts.is_empty() && report.layout_properties == ["rdlc", "word"] {
            Declares::WordAndRdlcProperties
        } else {
            Declares::Other
        },
        file: if report.layout_file_missing {
            LayoutFile::Missing
        } else {
            LayoutFile::Present
        },
        processing_only: report.processing_only,
        data_items,
        extensions,
        triggers,
    }
}

/// Whether the reportextension (object index) only adds columns to the report's data items: no trigger, data item, label or
/// `modify`.
pub(crate) fn adds_columns_only(program: &al_ir::Program, ext: usize) -> bool {
    program.objects[ext].report_ext.as_ref().is_some_and(|x| {
        x.on_pre.is_none()
            && x.on_post.is_none()
            && x.modifies.is_empty()
            && x.added.is_empty()
            && x.labels.is_empty()
            && !x.adds.is_empty()
    })
}

/// Whether every data item is as the false answers measured it (reports/html-email-body/RxdDeclFixtures.al:39-43): it declares
/// no OnPreDataItem, OnAfterGetRecord or OnPostDataItem, and every column's source is a stored field of the item's own table
/// (`column(RowId; Id)`, `column(RowTxt; Txt)`), so a run before the false answer runs no AL code.
pub(super) fn data_items_as_measured_for_false(
    schema: &Schema,
    tables: &[Option<std::sync::Arc<al_data::TableDef>>],
) -> bool {
    schema.report().items.iter().all(|i| {
        let table = i.table.and_then(|t| tables.get(t)?.as_deref());
        i.on_pre.is_none()
            && i.on_after_get.is_none()
            && i.on_post.is_none()
            && i.columns.iter().all(|c| {
                c.unsupported.is_none()
                    && c.value_unsupported.is_none()
                    && c.field.and_then(|p| table?.fields.get(p)).is_some_and(|f| f.stored)
            })
    })
}

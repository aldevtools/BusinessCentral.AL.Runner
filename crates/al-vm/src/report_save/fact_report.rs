//! The facts of the report a save renders: the default layout, how it is declared, the data items, the triggers and the
//! report extensions.

use super::Schema;
use super::facts::{DataItems, Decl, Declares, Extensions, Kind, Layout, LayoutFile, Triggers};

/// What the report declares, as the cells tell it apart.
pub(crate) struct ReportFacts {
    pub kind: Kind,
    pub layout: Layout,
    pub decl: Decl,
    pub declares: Declares,
    pub file: LayoutFile,
    pub processing_only: bool,
    pub data_items: DataItems,
    pub extensions: Extensions,
    pub triggers: Triggers,
    pub own_layout: bool,
    pub sole_layout: bool,
    pub init_triggers: bool,
}

pub(crate) fn report_facts(program: &al_ir::Program, schema: &Schema) -> ReportFacts {
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
    // `ReportLayout::None`: no DefaultRenderingLayout, no DefaultLayout, no layout property and no layout section (al-sema
    // `reports::default_layout`), so a lone WordLayout property or a rendering section without DefaultRenderingLayout is a
    // layout of its own.
    let own_layout = report.layout != al_ir::ReportLayout::None;
    let extensions = match report.extensions.as_slice() {
        [] => Extensions::None,
        [e] if adds_columns_only(program, *e) => Extensions::OneAddingColumns,
        // To a report with no layout of its own (SaveAsTriggerErrors.al "SAE No Layout Rpt").
        [e] if adds_nothing_to_the_run(program, *e) && report.extension_layouts == ["word"] && !own_layout => {
            Extensions::OneAddingWordLayout
        }
        _ => Extensions::Other,
    };
    let triggers = if report.on_post.is_some() {
        Triggers::OnPost
    } else if report.on_init.is_some() || report.on_pre.is_some() || report.on_pre_rendering.is_some() {
        Triggers::WithoutOnPost
    } else {
        Triggers::None
    };
    // corpus pull request 1579 (reports/layouts-saveas/SaveAsTriggerErrors.al): its reports declare one layout, no OnInitReport
    // and no request page; so do the processing-only report of streams/PnH6SaveAs.al:32 and the Html hash cells (`html_cell`).
    let sole_layout = report.layout_properties.is_empty()
        && report.rendering_layouts.len() == 1
        && report.extension_layouts.is_empty();
    // A request page trigger counts also when a reportextension adds it (`ir::Page::ext_triggers`), and any extension of
    // the request page counts, since none is measured (the runtime refuses an extension's OnInit where the page's runs,
    // `reports/request_unshown.rs`).
    let init_triggers = report.on_init.is_some()
        || program.objects[schema.object]
            .page
            .as_ref()
            .is_some_and(|p| !p.triggers.is_empty() || !p.ext_triggers.is_empty() || !p.extensions.is_empty());
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
        own_layout,
        sole_layout,
        init_triggers,
    }
}

/// Whether the reportextension (object index) only adds columns to the report's data items.
fn adds_columns_only(program: &al_ir::Program, ext: usize) -> bool {
    changes_no_trigger_item_or_label(program, ext, |x| !x.adds.is_empty())
}

/// Whether the reportextension adds no column either: what it adds then is layouts (`ir::Report::extension_layouts`).
fn adds_nothing_to_the_run(program: &al_ir::Program, ext: usize) -> bool {
    changes_no_trigger_item_or_label(program, ext, |x| x.adds.is_empty())
}

/// The reportextension has no trigger, data item, label or `modify`, and `adds` holds for its `add(Item)` blocks.
fn changes_no_trigger_item_or_label(
    program: &al_ir::Program,
    ext: usize,
    adds: impl Fn(&al_ir::ReportExt) -> bool,
) -> bool {
    program.objects[ext].report_ext.as_ref().is_some_and(|x| {
        x.on_pre.is_none()
            && x.on_post.is_none()
            && x.modifies.is_empty()
            && x.added.is_empty()
            && x.labels.is_empty()
            && adds(x)
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

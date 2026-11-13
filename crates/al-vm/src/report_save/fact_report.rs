//! The facts of the report a save renders: the default layout, how it is declared, the data items, the triggers and the
//! report extensions.

use super::Schema;
use super::facts::{DataItems, Decl, Extensions, Kind, Layout, LayoutFile, Triggers};

/// What the report declares, as the cells tell it apart.
pub(super) struct ReportFacts {
    pub kind: Kind,
    pub layout: Layout,
    pub decl: Decl,
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
        _ => DataItems::Other,
    };
    let extensions = match report.extensions.as_slice() {
        [] => Extensions::None,
        [e] => match program.objects[*e].report_ext.as_ref() {
            Some(x)
                if x.on_pre.is_none()
                    && x.on_post.is_none()
                    && x.modifies.is_empty()
                    && x.added.is_empty()
                    && x.labels.is_empty()
                    && !x.adds.is_empty() =>
            {
                Extensions::OneAddingColumns
            }
            _ => Extensions::Other,
        },
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

//! What a run takes from the manifests and the package readers into the compiled program's apps.

use super::Manifest;
use std::sync::Arc;

/// Sets the manifest facts (publisher, version, resources, layout files, ...) on `program`'s apps, `manifests[i]` on app `i`,
/// and the reader of the layout files of each app in `layout_sources` (by app id).
pub(super) fn apply(
    program: &mut Arc<al_ir::Program>,
    manifests: &[Manifest],
    layout_sources: &[(String, al_ir::LayoutSourceRef)],
) {
    let program = Arc::make_mut(program);
    for (app, m) in program.apps.iter_mut().zip(manifests) {
        app.publisher = m.publisher.clone();
        app.version = m.version.clone();
        app.resources = m.resources.clone();
        app.include_source = m.include_source;
        app.target = (!m.target.is_empty()).then(|| m.target.clone());
        app.logo = m.logo.clone();
        app.description = m.description.clone();
        app.layout_files = m.layout_files.clone();
        (app.translations, app.package_data, app.package_data_files) = m.shipped_files();
        app.translation_files = m.translation_files.clone();
        app.translation_source = m.translation_source.clone();
    }
    for (id, source) in layout_sources {
        if let Some(app) = program.apps.iter_mut().find(|a| a.id.eq_ignore_ascii_case(id)) {
            app.layout_source = Some(source.clone());
        }
    }
}

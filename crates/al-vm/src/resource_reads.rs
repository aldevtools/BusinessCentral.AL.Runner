//! The resource files of the apps that AL code of a session read (`NavApp.GetResource*`, `NavApp.ListResources`), and
//! the texts it got. al-testrun compares them: install code that ran for one entry of a run serves another entry only when
//! that entry's texts of the files it read are the same (`install_once.rs`, ADR-028 decision 11), and a login start is
//! replayed only into a session whose texts of the files its login read are the same (`login_start.rs`, ADR-073
//! amendment 2026-11-25).
//!
//! `Ctx::app_resource` notes one file, `Ctx::app_resources` the whole app. [`TextsRead`] owns the rule "the texts of
//! what was read are still the same". Entry points: `Session::resource_reads`, `ResourceReads`, `TextsRead`.

use crate::{Engine, ResourceOverrides, Session};
use std::collections::{BTreeMap, BTreeSet};

/// Program app index to the files of that app whose content AL code read, by name; `None`: the whole list of the app's
/// files was handed out (`Ctx::app_resources`), so every file counts as read.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct ResourceReads(BTreeMap<usize, Option<BTreeSet<String>>>);

/// The entries of a run's `ResourceOverrides` that replace a file AL code read ([`ResourceReads::texts`]).
type ReadTexts = Vec<(usize, Vec<(String, Vec<u8>)>)>;

impl ResourceReads {
    /// The replaced texts of the files read, under `overrides`. A file that is not replaced is read from the program,
    /// which every entry of a run shares, so equal texts under two runs' overrides give the same reads.
    fn texts(&self, overrides: &ResourceOverrides) -> ReadTexts {
        self.0
            .iter()
            .map(|(app, names)| {
                let replaced = overrides.get(app).map_or_else(Vec::new, |files| {
                    files
                        .iter()
                        .filter(|(n, _)| names.as_ref().is_none_or(|read| read.contains(n)))
                        .cloned()
                        .collect()
                });
                (*app, replaced)
            })
            .collect()
    }
}

/// The resource files AL code read ([`ResourceReads`]) with their texts under the run's overrides when it read them, by
/// content. Code that read them leaves the same for an entry whose texts of those files are the same
/// ([`TextsRead::same_now`]); a file it did not read may have any text, and code that read none leaves the same under
/// any texts.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct TextsRead {
    reads: ResourceReads,
    texts: ReadTexts,
}

impl TextsRead {
    /// `reads` with their texts under `engine`'s overrides now (`Engine::resource_overrides`); none when the overrides
    /// cannot be read.
    pub fn now(reads: ResourceReads, engine: &Engine) -> TextsRead {
        let texts = (engine.resource_overrides.read())
            .map(|o| reads.texts(&o))
            .unwrap_or_default();
        TextsRead { reads, texts }
    }

    /// Whether `engine`'s overrides now give the files read the texts they had: `false` when the overrides cannot be
    /// read.
    pub fn same_now(&self, engine: &Engine) -> bool {
        (engine.resource_overrides.read()).is_ok_and(|o| self.reads.texts(&o) == self.texts)
    }
}

impl Session {
    /// The resource files AL code of this session read (`Engine::resources_read` says whether any session read one).
    pub fn resource_reads(&self) -> ResourceReads {
        self.resource_reads.borrow().clone()
    }

    /// The app that owns the running code, by program index, with its name as BC gives it
    /// (session-system/W5NavAppResources.al:GetResourceAsText_NameMatching: `<name> by <publisher> <version>`).
    fn resource_app(&self) -> Option<(usize, &al_ir::AppInfo, String)> {
        let obj = *self.frames.last()?;
        let index = self.engine.program.objects[obj].app as usize;
        let app = self.engine.program.apps.get(index)?;
        self.engine
            .resources_read
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let label = format!("{} by {} {}", app.name, app.publisher, app.version);
        Some((index, app, label))
    }

    /// The content of the file `name` of app `index`: the run's text when it replaces it (`ResourceOverrides`), else the
    /// program's `content`.
    fn resource_content(&self, index: usize, name: &str, content: &[u8]) -> Vec<u8> {
        let overrides = self.engine.resource_overrides.read().ok();
        let replaced = overrides.as_ref().and_then(|o| o.get(&index));
        replaced
            .and_then(|r| r.iter().find(|(n, _)| n == name))
            .map_or(content, |(_, c)| c)
            .to_vec()
    }

    /// Every file of the running app with its content (`Ctx::app_resources`); every file counts as read.
    pub(crate) fn running_app_resources(&self) -> Option<al_builtins::native::AppResources> {
        let (index, app, label) = self.resource_app()?;
        self.resource_reads.borrow_mut().0.insert(index, None);
        let resources = app
            .resources
            .iter()
            .map(|(name, content)| (name.clone(), self.resource_content(index, name, content)))
            .collect();
        Some((label, resources))
    }

    /// The file `name` of the running app (`Ctx::app_resource`); only that file counts as read.
    pub(crate) fn running_app_resource(&self, name: &str) -> Option<(String, Option<Vec<u8>>)> {
        let (index, app, label) = self.resource_app()?;
        if let Some(read) = self
            .resource_reads
            .borrow_mut()
            .0
            .entry(index)
            .or_insert_with(|| Some(BTreeSet::new()))
        {
            read.insert(name.to_string());
        }
        let content = app
            .resources
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, content)| self.resource_content(index, name, content));
        Some((label, content))
    }
}

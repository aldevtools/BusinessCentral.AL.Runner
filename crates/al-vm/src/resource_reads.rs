//! The resource files of the apps that AL code of a session read (`NavApp.GetResource*`, `NavApp.ListResources`), and
//! the texts it got. al-testrun compares them: install code that ran for one entry of a run serves another entry only when
//! that entry's texts of the files it read are the same (`install_once.rs`, ADR-028 decision 11), and a login start is
//! replayed only into a session whose texts of the files its login read are the same (`login_start.rs`, ADR-073
//! amendment 2026-11-25).
//!
//! `Ctx::app_resource` notes one file, `Ctx::app_resources` the whole app. [`TextsRead`] owns the rule "the texts of
//! what was read are still the same". Entry points: `Session::resource_reads`, `ResourceReads`, `TextsRead`.

use crate::{Engine, ResourceOverrides, Session};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};

/// Program app index to the files of that app whose content AL code read, by name; `None`: the whole list of the app's
/// files was handed out (`Ctx::app_resources`), so every file counts as read.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct ResourceReads(BTreeMap<usize, Option<BTreeSet<String>>>);

/// The entries of a run's `ResourceOverrides` that replace a file AL code read ([`ResourceReads::texts`]): per app, each
/// replaced file by name with the SHA-256 digest of its text.
type ReadTexts = Vec<(usize, Vec<(String, [u8; 32])>)>;

impl ResourceReads {
    /// The digests of the replaced texts of the files read, under `overrides`. A file that is not replaced is read from
    /// the program, which every entry of a run shares, so equal texts under two runs' overrides give the same reads.
    fn texts(&self, overrides: &ResourceOverrides) -> ReadTexts {
        self.0
            .iter()
            .map(|(app, names)| {
                let replaced = overrides.get(app).map_or_else(Vec::new, |files| {
                    files
                        .iter()
                        .filter(|(n, _)| names.as_ref().is_none_or(|read| read.contains(n)))
                        .map(|(n, text)| (n.clone(), sha2::Sha256::digest(text).into()))
                        .collect()
                });
                (*app, replaced)
            })
            .collect()
    }
}

/// The resource files AL code read ([`ResourceReads`]) with their texts under the run's overrides when it read them, each
/// as its SHA-256 digest: what a kept start or install-once holds does not grow with the texts. Code that read them leaves the same for an entry whose texts of those files are the same
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

    /// Whether `engine`'s overrides now give the files read the texts they had (their digests, taken in place; no text
    /// is copied): `false` when the overrides cannot be read.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A [`TextsRead`] of file `f.txt` of app 0, read under the text `text`.
    fn read_under(text: Vec<u8>) -> (TextsRead, std::sync::Arc<Engine>) {
        let engine = Engine::new(al_ir::Program::default(), Vec::new());
        let file = |t: Vec<u8>| std::sync::Arc::new(vec![("f.txt".to_string(), t)]);
        engine.resource_overrides.write().unwrap().insert(0, file(text));
        let reads = ResourceReads([(0, Some(["f.txt".to_string()].into()))].into());
        (TextsRead::now(reads, &engine), engine)
    }

    // Review of al-runner PR 9074: a start keeps what its login read outside the starts' byte meter, so it holds a
    // SHA-256 digest per file read, never the text: what it holds does not grow with the text, and another text of the
    // same size is still another read.
    #[test]
    fn a_large_text_read_is_held_as_a_digest() {
        let (small, _) = read_under(vec![b'a'; 1]);
        let (large, engine) = read_under(vec![b'a'; 1 << 20]);
        assert!(
            format!("{large:?}").len() < 1024,
            "{} bytes",
            format!("{large:?}").len()
        );
        assert_eq!(format!("{small:?}").len(), format!("{large:?}").len());
        assert!(large.same_now(&engine));
        let mut other = vec![b'a'; 1 << 20];
        other[7] = b'b';
        let (_, other_engine) = read_under(other);
        assert!(!large.same_now(&other_engine));
    }
}

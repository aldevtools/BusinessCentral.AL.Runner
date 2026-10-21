//! The key of a cache entry: every input of the compile of the dependency set.

use sha2::{Digest, Sha256};

/// The format integer of the entry file. Bump it when the layout of the file or of a type it
/// stores changes. (Any change to the compiler already changes `Key::runtime_build`.)
pub const FORMAT: u32 = 14;

/// One dependency app, as the compile saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppKey {
    pub id: String,
    pub name: String,
    pub dependencies: Vec<String>,
    pub publisher: String,
    pub version: String,
    /// The preprocessor symbols the app is compiled with: those of its own app.json, sorted. The
    /// project's symbols are not an input of a dependency layer.
    pub symbols: Vec<String>,
    /// A hash of the app's source, made by the caller: of every path and byte, or, for a zip package,
    /// of the name, size and CRC-32 of every entry (the zip directory), which needs no read of the data.
    pub source: [u8; 32],
}

/// Everything the lowered form of the dependency set depends on. The digest destructures the
/// struct without `..`, so a field added here and not hashed does not compile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// [`FORMAT`] when the key is made.
    pub format: u32,
    /// A hash of the running executable: the compiler, the builtins catalog and every crate.
    pub runtime_build: [u8; 32],
    /// The target BC version and runtime (`Target`, ADR-015), as text. Empty: none.
    pub target: String,
    /// The apps of this layer, in program order (dependencies first). The first layer holds the
    /// platform and Microsoft's apps; every other layer holds the apps above it (ADR-022, layers).
    pub apps: Vec<AppKey>,
    /// The digest of the key of the layer below, all zeros for the first layer. A layer was
    /// compiled with the layers below it present, so a change anywhere below changes this.
    pub below: [u8; 32],
}

fn put(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

fn put_list<S: AsRef<[u8]>>(h: &mut Sha256, items: &[S]) {
    h.update((items.len() as u64).to_le_bytes());
    for i in items {
        put(h, i.as_ref());
    }
}

impl Key {
    /// A key with the current [`FORMAT`].
    pub fn new(runtime_build: [u8; 32], target: String, apps: Vec<AppKey>) -> Key {
        Key::layered(runtime_build, target, apps, [0; 32])
    }

    /// A key for a layer above another: `below` is the digest of the key of the layer below.
    pub fn layered(runtime_build: [u8; 32], target: String, apps: Vec<AppKey>, below: [u8; 32]) -> Key {
        Key {
            format: FORMAT,
            runtime_build,
            target,
            apps,
            below,
        }
    }

    /// The names of the fields, for the test that every field has a miss test.
    pub const FIELDS: [&'static str; 5] = ["format", "runtime_build", "target", "apps", "below"];

    /// The digest of the key. Equal keys give equal digests; a key that differs in any field
    /// gives a different one.
    pub fn digest(&self) -> [u8; 32] {
        let Key {
            format,
            runtime_build,
            target,
            apps,
            below,
        } = self;
        let mut h = Sha256::new();
        h.update(b"al-runtime compile cache key");
        h.update(format.to_le_bytes());
        put(&mut h, runtime_build);
        put(&mut h, target.as_bytes());
        put(&mut h, below);
        h.update((apps.len() as u64).to_le_bytes());
        for a in apps {
            let AppKey {
                id,
                name,
                dependencies,
                publisher,
                version,
                symbols,
                source,
            } = a;
            put(&mut h, id.as_bytes());
            put(&mut h, name.as_bytes());
            put_list(&mut h, dependencies);
            put(&mut h, publisher.as_bytes());
            put(&mut h, version.as_bytes());
            put_list(&mut h, symbols);
            put(&mut h, source);
        }
        h.finalize().into()
    }

    /// The digest as lower-case hex.
    pub fn hex(&self) -> String {
        self.digest().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The file name of the entry for this key.
    pub fn file_name(&self) -> String {
        format!("{}-deps-{}.alcc", build_tag(&self.runtime_build), &self.hex()[..32])
    }
}

/// The first 16 hex digits of a runtime build id. Every entry file name starts with it, so a cleanup can tell
/// which build wrote a file from the name alone (ADR-022, "Cleanup of old builds").
pub fn build_tag(build: &[u8; 32]) -> String {
    build[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// The build tag a cache file name starts with: `<16 hex>-<rest>`. `None` for a name made before the tag existed.
pub fn tag_of_file_name(name: &str) -> Option<&str> {
    let (tag, rest) = name.split_once('-')?;
    (tag.len() == 16 && !rest.is_empty() && tag.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        .then_some(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample() -> Key {
        Key::new(
            [7; 32],
            "BC 27.5 runtime 16.0".into(),
            vec![
                AppKey {
                    id: "a".into(),
                    name: "Base".into(),
                    dependencies: vec!["s".into()],
                    publisher: "Microsoft".into(),
                    version: "27.5.0.0".into(),
                    symbols: vec![],
                    source: [1; 32],
                },
                AppKey {
                    id: "b".into(),
                    name: "App".into(),
                    dependencies: vec!["a".into()],
                    publisher: "Microsoft".into(),
                    version: "27.5.0.0".into(),
                    symbols: vec![],
                    source: [2; 32],
                },
            ],
        )
    }

    /// Each input, changed alone, misses. `FIELDS` names the fields of `Key` and this table has
    /// one row per name, so a new field needs a row (and the digest needs to hash it to compile).
    #[test]
    fn every_input_of_the_key_misses_when_it_changes() {
        type Change = fn(&mut Key);
        let changes: Vec<(&str, Change)> = vec![
            ("format", |k| k.format += 1),
            ("runtime_build", |k| k.runtime_build[31] ^= 1),
            ("target", |k| k.target = "BC 28.0 runtime 17.0".into()),
            ("apps", |k| k.apps.reverse()),
            ("below", |k| k.below[5] ^= 1),
        ];
        let names: Vec<&str> = changes.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, Key::FIELDS, "one miss test per field of the key");
        let base = sample();
        for (name, change) in changes {
            let mut k = base.clone();
            change(&mut k);
            assert_ne!(k.digest(), base.digest(), "changing `{name}` must change the digest");
        }
    }

    /// Each input of one app, changed alone, misses.
    #[test]
    fn every_input_of_an_app_misses_when_it_changes() {
        type Change = fn(&mut AppKey);
        let changes: Vec<(&str, Change)> = vec![
            ("id", |a| a.id = "z".into()),
            ("name", |a| a.name = "Other".into()),
            ("dependencies", |a| a.dependencies.push("x".into())),
            ("publisher", |a| a.publisher = "Other".into()),
            ("version", |a| a.version = "27.5.0.1".into()),
            ("symbols", |a| a.symbols.push("EXTRA".into())),
            ("source", |a| a.source[0] ^= 1),
        ];
        let base = sample();
        for (name, change) in changes {
            let mut k = base.clone();
            change(&mut k.apps[0]);
            assert_ne!(
                k.digest(),
                base.digest(),
                "changing `{name}` of an app must change the digest"
            );
        }
        // A fresh key with the same inputs hits.
        assert_eq!(sample().digest(), base.digest());
    }

    #[test]
    fn fields_do_not_run_into_each_other() {
        let mut a = sample();
        let mut b = sample();
        a.apps[0].id = "ab".into();
        a.apps[0].name = "c".into();
        b.apps[0].id = "a".into();
        b.apps[0].name = "bc".into();
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn every_entry_file_name_starts_with_the_build_tag() {
        let build = [0xab; 32];
        let k = Key::new(build, "t".into(), vec![]);
        let tag = build_tag(&build);
        assert_eq!(tag, "abababababababab");
        for name in [k.file_name(), crate::index_file_name(&k)] {
            assert_eq!(tag_of_file_name(&name), Some(tag.as_str()), "{name}");
        }
        assert_eq!(tag_of_file_name("deps-0123.alcc"), None, "a name from before the tag");
        assert_eq!(tag_of_file_name(&format!("{}.albr", "c".repeat(64))), None);
        assert_ne!(
            k.file_name(),
            Key::new([0xcd; 32], "t".into(), vec![]).file_name(),
            "another build, another name"
        );
    }
}

//! AL grammar.
//!
//! Grammar entry module.

mod declarations;
mod expressions;
mod objects;
mod statements;

pub(crate) use declarations::type_ref;
pub(crate) use objects::source_file;

#[cfg(test)]
pub(crate) mod test_util {
    use crate::parser::{Parser, parse_entry};
    use crate::{SyntaxKind, debug_tree};

    /// Parses `src` with `entry` and returns the dump of the entry's node(s),
    /// asserting no errors.
    pub fn dump(src: &str, entry: fn(&mut Parser)) -> String {
        let p = parse_entry(src, &Default::default(), &mut crate::TreeCache::default(), entry);
        assert!(
            p.errors.is_empty(),
            "errors for {src:?}: {:?}\n{}",
            p.errors,
            debug_tree(&p.syntax())
        );
        assert_eq!(p.syntax().text().to_string(), src, "lossless");
        let root = p.syntax();
        assert_eq!(root.kind(), SyntaxKind::SourceFile);
        root.children().map(|c| debug_tree(&c)).collect::<Vec<_>>().join(" ")
    }

    pub fn errors(src: &str, entry: fn(&mut Parser)) -> usize {
        let p = parse_entry(src, &Default::default(), &mut crate::TreeCache::default(), entry);
        assert_eq!(p.syntax().text().to_string(), src, "lossless even with errors");
        p.errors.len()
    }
}

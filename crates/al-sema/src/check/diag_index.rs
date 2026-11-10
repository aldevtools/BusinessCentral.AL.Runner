//! Finds the diagnostics at one position of one file without scanning the whole list: lowering asks once per call it
//! lowers whether the checker refused that call (AR1006, AR1008, AR1009, AL0666, AR1007).

use crate::workspace::Diagnostic;

/// The positions of a diagnostic list in the order of (file path, offset), with list order kept among equal positions.
/// It holds indices only and answers for the list it was built from. The file path alone decides the file, not the app,
/// as the scans it replaced did.
#[derive(Debug, Default)]
pub struct DiagIndex {
    order: Vec<u32>,
}

impl DiagIndex {
    pub fn build(diags: &[Diagnostic]) -> DiagIndex {
        let mut order: Vec<u32> = (0..diags.len() as u32).collect();
        // A stable sort: among the diagnostics at one position, list order stays.
        order.sort_by(|&a, &b| {
            let (a, b) = (&diags[a as usize], &diags[b as usize]);
            (a.file.as_str(), a.offset).cmp(&(b.file.as_str(), b.offset))
        });
        DiagIndex { order }
    }

    /// The first diagnostic of `diags` (in list order) at `offset` of `file` whose code is one of `codes`.
    pub fn find<'a>(
        &self,
        diags: &'a [Diagnostic],
        file: &str,
        offset: usize,
        codes: &[&str],
    ) -> Option<&'a Diagnostic> {
        debug_assert_eq!(
            self.order.len(),
            diags.len(),
            "the index answers for the list it was built from"
        );
        let key = |i: &u32| {
            let d = &diags[*i as usize];
            (d.file.as_str(), d.offset)
        };
        let start = self.order.partition_point(|i| key(i) < (file, offset));
        self.order[start..]
            .iter()
            .take_while(|i| key(i) == (file, offset))
            .map(|&i| &diags[i as usize])
            .find(|d| codes.contains(&d.code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Diagnostic;

    fn d(file: &str, app: usize, offset: usize, code: &'static str, message: &str) -> Diagnostic {
        Diagnostic {
            file: file.into(),
            app,
            offset,
            code,
            message: message.into(),
        }
    }

    /// The linear scan lowering did before the index.
    fn scan<'a>(diags: &'a [Diagnostic], file: &str, offset: usize, codes: &[&str]) -> Option<&'a Diagnostic> {
        diags
            .iter()
            .find(|d| codes.contains(&d.code) && d.offset == offset && d.file == file)
    }

    #[test]
    fn finds_what_the_linear_scan_found() {
        // Unsorted on purpose, several codes at one offset, the same offset in two files, the same file in two apps,
        // and two diagnostics with the same code at one position: the first in list order wins.
        let diags = vec![
            d("b.al", 0, 10, "AR1008", "b10 1008"),
            d("a.al", 0, 10, "AR1009", "a10 1009"),
            d("a.al", 0, 10, "AR1008", "a10 1008 first"),
            d("a.al", 1, 10, "AR1008", "a10 1008 other app"),
            d("a.al", 0, 5, "AL0666", "a5 0666"),
            d("a.al", 0, 10, "AR1008", "a10 1008 second"),
            d("a.al", 0, 5, "AR1007", "a5 1007"),
            d("c.al", 0, 0, "AR1006", "c0 1006"),
            d("a.al", 0, 20, "AR0000", "a20 parse"),
        ];
        let index = DiagIndex::build(&diags);
        let code_sets: [&[&str]; 6] = [
            &["AR1008"],
            &["AR1009"],
            &["AR1006"],
            &["AL0666", "AR1007"],
            &["AR1007", "AL0666"],
            &["AR0000"],
        ];
        for file in ["a.al", "b.al", "c.al", "d.al", ""] {
            for offset in [0, 5, 10, 11, 20, 99] {
                for codes in code_sets {
                    let want = scan(&diags, file, offset, codes);
                    let got = index.find(&diags, file, offset, codes);
                    assert_eq!(
                        got.map(|g| g as *const Diagnostic),
                        want.map(|w| w as *const Diagnostic),
                        "{file}:{offset} {codes:?}: {got:?} against {want:?}"
                    );
                }
            }
        }
        assert_eq!(
            index.find(&diags, "a.al", 10, &["AR1008"]).unwrap().message,
            "a10 1008 first"
        );
        assert_eq!(
            index.find(&diags, "a.al", 5, &["AR1007", "AL0666"]).unwrap().message,
            "a5 0666"
        );
        assert!(DiagIndex::build(&[]).find(&[], "a.al", 0, &["AR1008"]).is_none());
    }
}

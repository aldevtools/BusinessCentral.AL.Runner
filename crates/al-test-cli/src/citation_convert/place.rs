//! What a cited line of a corpus file names at one pin, for `--convert-citations`: the file header, an object, one of
//! its members (with the cell when the line is one observation of a test), or nothing (past the end, between objects).
//! A range names every member it touches, or the codeunit when it spans most of its tests. Entry points: `File`,
//! `File::targets`.

use al_testrun::discover::objects::{self, ObjectDecl};

/// Observer methods whose first argument is the cell's name, as the corpus calls them (`Observer.Expect('cell', ..)`).
const CELL_METHODS: &[&str] = &["Expect", "Observe", "ObserveError", "Ignore"];
/// A range that spans more than this many tests, and at least half the tests of the codeunit, cites the codeunit.
const MANY_TESTS: usize = 3;

/// One corpus file at one pin.
pub struct File<'a> {
    text: &'a str,
    code: String,
    /// The byte where each line starts (line 1 at index 0).
    starts: Vec<usize>,
    pub objects: Vec<ObjectDecl>,
}

/// What a cited line names; indexes are into `File::objects` and its members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Before the first object: the file's first test codeunit (else its first object).
    Header(usize),
    Object(usize),
    Member { object: usize, member: usize, cell: Option<String> },
}

impl<'a> File<'a> {
    pub fn new(text: &'a str) -> File<'a> {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .filter(|&s| s < text.len())
            .collect();
        File {
            text,
            code: objects::code_text(text),
            starts,
            objects: objects::objects(text),
        }
    }

    /// The object a file citation without lines names: the first test codeunit, else the first object.
    pub fn main_object(&self) -> Option<usize> {
        self.objects
            .iter()
            .position(|o| o.test_codeunit)
            .or((!self.objects.is_empty()).then_some(0))
    }

    /// What the line ranges name, in order and without repeats. A single line inside a test gets its cell when the
    /// line holds one observation; a range keeps its members (its blank and comment lines name the object) and cites the
    /// codeunit when it spans more than [`MANY_TESTS`] tests and at least half of them.
    pub fn targets(&self, ranges: &[(u32, u32)]) -> Result<Vec<Target>, String> {
        let mut out: Vec<Target> = Vec::new();
        for &(first, last) in ranges {
            let mut here = Vec::new();
            for line in first..=last {
                here.push(self.place(line as usize)?);
            }
            if first == last
                && let Some(Target::Member { object, member, cell }) = here.first_mut()
                && self.objects[*object].members[*member].test
            {
                *cell = self.cell(first as usize);
            }
            for t in self.narrow(here) {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
        Ok(out)
    }

    /// The targets of one range: members over objects over the header, the codeunit for most of its tests.
    fn narrow(&self, here: Vec<Target>) -> Vec<Target> {
        let mut members: Vec<Target> = Vec::new();
        for t in &here {
            if matches!(t, Target::Member { .. }) && !members.contains(t) {
                members.push(t.clone());
            }
        }
        if members.is_empty() {
            let mut rest: Vec<Target> = Vec::new();
            for t in here {
                if !rest.contains(&t) {
                    rest.push(t);
                }
            }
            if rest.iter().any(|t| matches!(t, Target::Object(_))) {
                rest.retain(|t| matches!(t, Target::Object(_)));
            }
            return rest;
        }
        let tests = |o: usize| self.objects[o].members.iter().filter(|m| m.test).count();
        if let Some(Target::Member { object, .. }) = members.first() {
            let o = *object;
            let spanned = members
                .iter()
                .filter(|t| matches!(t, Target::Member { object, member, .. } if *object == o && self.objects[o].members[*member].test))
                .count();
            if spanned > MANY_TESTS && spanned * 2 >= tests(o) && members.len() == spanned {
                return vec![Target::Object(o)];
            }
        }
        members
    }

    /// What line `line` (1-based) names.
    fn place(&self, line: usize) -> Result<Target, String> {
        if line == 0 || line > self.starts.len() {
            return Err(format!("line {line} is past the end of the file ({} lines)", self.starts.len()));
        }
        let line_of = |at: usize| self.starts.partition_point(|&s| s <= at);
        let Some(o) = self
            .objects
            .iter()
            .position(|o| line_of(o.span.start) <= line && line <= line_of(o.span.end.saturating_sub(1)))
        else {
            return match self.objects.first() {
                Some(first) if line < line_of(first.span.start) => {
                    self.main_object().map(Target::Header).ok_or_else(|| "the file has no object".into())
                }
                Some(_) => Err(format!("line {line} is outside every object")),
                None => Err("the file has no object".into()),
            };
        };
        let obj = &self.objects[o];
        let header = line_of(obj.span.start);
        for (k, m) in obj.members.iter().enumerate() {
            let keyword = line_of(m.span.start);
            let mut top = keyword;
            while top - 1 > header && self.leads_member(top - 1) {
                top -= 1;
            }
            let last = line_of(m.span.end.saturating_sub(1)).max(keyword);
            let bottom = (keyword..=last).rev().find(|&l| self.ends_body(l)).unwrap_or(keyword);
            if top <= line && line <= bottom {
                return Ok(Target::Member {
                    object: o,
                    member: k,
                    cell: None,
                });
            }
        }
        Ok(Target::Object(o))
    }

    /// The text and the code (comments and string texts blanked) of line `line`.
    fn line(&self, line: usize) -> (&str, &str) {
        let start = self.starts[line - 1];
        let end = self.starts.get(line).copied().unwrap_or(self.text.len());
        (&self.text[start..end], &self.code[start..end])
    }

    /// A line just above a member's keyword that belongs to it: an attribute (`[Test]`) or a comment.
    fn leads_member(&self, line: usize) -> bool {
        let (text, code) = self.line(line);
        let code = code.trim();
        code.starts_with('[') || (code.is_empty() && !text.trim().is_empty())
    }

    /// The line ends a member's body: its code holds `end;` (or is `end`).
    fn ends_body(&self, line: usize) -> bool {
        let code = self.line(line).1.to_ascii_lowercase();
        let code = code.trim();
        code == "end"
            || code.match_indices("end").any(|(i, _)| {
                let before = code[..i].bytes().last();
                !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
                    && code[i + 3..].trim_start().starts_with(';')
            })
    }

    /// The cell of line `line`: the first argument of its one observation call, when that is a string literal.
    fn cell(&self, line: usize) -> Option<String> {
        let start = self.starts[line - 1];
        let (_, code) = self.line(line);
        let calls: Vec<usize> = CELL_METHODS
            .iter()
            .flat_map(|m| {
                let call = format!(".{m}(");
                code.match_indices(call.as_str())
                    .map(|(i, _)| i + m.len() + 2)
                    .collect::<Vec<_>>()
            })
            .collect();
        let [open] = calls.as_slice() else { return None };
        let arg = open + code[*open..].len() - code[*open..].trim_start().len();
        let n = crate::citation::literal(&self.text[start + arg..])?;
        (n > 2).then(|| self.text[start + arg..start + arg + n].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "// The file header.
codeunit 50100 \"LT Place\"
{
    Subtype = Test;

    [Test]
    procedure First()
    begin
        Observer.Expect('one', 1);
        Observer.Expect('two', 2); Observer.Expect('three', 3);
    end;

    // A helper.
    local procedure Helper()
    begin
    end;

    [Test]
    procedure Second()
    begin
    end;
}

table 50101 \"LT Place Table\"
{
    fields { field(1; A; Integer) { } }
}
";

    fn t(lines: &[(u32, u32)]) -> Result<Vec<Target>, String> {
        File::new(SRC).targets(lines)
    }
    fn member(member: usize, cell: Option<&str>) -> Target {
        Target::Member {
            object: 0,
            member,
            cell: cell.map(str::to_string),
        }
    }

    #[test]
    fn a_line_names_the_header_its_object_or_its_member() {
        assert_eq!(t(&[(1, 1)]), Ok(vec![Target::Header(0)]));
        assert_eq!(t(&[(4, 4)]), Ok(vec![Target::Object(0)]));
        // The attribute line belongs to its test; the comment above a helper to the helper.
        assert_eq!(t(&[(6, 6)]), Ok(vec![member(0, None)]));
        assert_eq!(t(&[(13, 13)]), Ok(vec![member(1, None)]));
        assert_eq!(t(&[(16, 16)]), Ok(vec![member(1, None)]));
        assert_eq!(t(&[(26, 26)]), Ok(vec![Target::Object(1)]));
    }

    #[test]
    fn a_line_with_one_observation_names_its_cell() {
        assert_eq!(t(&[(9, 9)]), Ok(vec![member(0, Some("'one'"))]));
        // Two observations on one line: the test only.
        assert_eq!(t(&[(10, 10)]), Ok(vec![member(0, None)]));
        // A range: no cell.
        assert_eq!(t(&[(9, 10)]), Ok(vec![member(0, None)]));
    }

    #[test]
    fn a_range_names_each_member_it_touches() {
        // Blank and comment lines between the members name no object.
        assert_eq!(t(&[(7, 20)]), Ok(vec![member(0, None), member(1, None), member(2, None)]));
        assert_eq!(t(&[(9, 9), (20, 20)]), Ok(vec![member(0, Some("'one'")), member(2, None)]));
    }

    #[test]
    fn a_line_past_the_end_or_between_objects_is_an_error() {
        assert!(t(&[(99, 99)]).unwrap_err().contains("past the end"));
        assert!(t(&[(23, 23)]).unwrap_err().contains("outside every object"));
    }

    #[test]
    fn a_range_over_most_tests_names_the_codeunit() {
        let tests: String = (1..=5)
            .map(|k| format!("    [Test]\n    procedure T{k}()\n    begin\n    end;\n"))
            .collect();
        let src = format!("codeunit 50100 \"LT Many\"\n{{\n    Subtype = Test;\n{tests}}}\n");
        assert_eq!(File::new(&src).targets(&[(4, 22)]), Ok(vec![Target::Object(0)]));
        assert_eq!(File::new(&src).targets(&[(4, 11)]).unwrap().len(), 2);
    }
}

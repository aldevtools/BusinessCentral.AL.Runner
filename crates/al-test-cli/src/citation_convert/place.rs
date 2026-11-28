//! What a cited line of a corpus file names at one pin, for `--convert-citations`: the file header, an object, one of
//! its members (with the cell when the line is one observation of a test), or nothing (past the end, between objects).
//! A range names every member it touches, or the codeunit when it spans most of its tests. Entry points: `File`,
//! `File::targets`.

use al_testrun::discover::objects::{self, ObjectDecl};

/// A range that spans more than this many tests, and at least half the tests of the codeunit, cites the codeunit.
const MANY_TESTS: usize = 3;

/// One corpus file at one pin.
pub struct File<'a> {
    text: &'a str,
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
    Member {
        object: usize,
        member: usize,
        cell: Option<String>,
    },
}

impl<'a> File<'a> {
    pub fn new(text: &'a str) -> File<'a> {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .filter(|&s| s < text.len())
            .collect();
        File {
            text,
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
            if first == last {
                self.lone_line(first as usize)?;
            }
            let mut here = Vec::new();
            for line in first..=last {
                here.push(self.place(line as usize)?);
            }
            match here.first_mut() {
                Some(Target::Member { object, member, cell })
                    if first == last && self.objects[*object].members[*member].test =>
                {
                    *cell = self.cell(first as usize);
                }
                // A test codeunit's facts live in its tests: a line between them was likely counted at another pin.
                Some(Target::Object(o)) if first == last && self.objects[*o].test_codeunit => {
                    return Err(format!("line {first} is outside every member of a test codeunit"));
                }
                _ => {}
            }
            for t in self.narrow(here) {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
        Ok(out)
    }

    /// A single cited line that names nothing by itself (blank, or a lone brace) is an error: the line count was
    /// likely taken at another pin.
    fn lone_line(&self, line: usize) -> Result<(), String> {
        let Some(&start) = self.starts.get(line.wrapping_sub(1)) else {
            return Ok(());
        };
        let end = self.starts.get(line).copied().unwrap_or(self.text.len());
        match self.text[start..end].trim() {
            "" => Err(format!("line {line} is blank")),
            "{" | "}" => Err(format!("line {line} is a lone brace")),
            _ => Ok(()),
        }
    }

    /// The bytes a target covers: its member, else its object.
    fn span(&self, t: &Target) -> std::ops::Range<usize> {
        match t {
            Target::Member { object, member, .. } => self.objects[*object].members[*member].span.clone(),
            Target::Object(o) | Target::Header(o) => self.objects[*o].span.clone(),
        }
    }

    /// The first of `quotes` (from the citing text) that the file has outside the targets: a cell literal (`'x'`)
    /// of the file in no target's span, or a member name (any other quote) of the file that no target is or holds.
    /// Such a citation names one place in prose and another by its line, so it is left for a hand fix.
    pub fn quoted_elsewhere(&self, targets: &[Target], quotes: &[String]) -> Option<String> {
        let file_cells = objects::cells(self.text, 0..self.text.len());
        let has_member = |o: &ObjectDecl, q: &str| o.members.iter().any(|m| m.name.eq_ignore_ascii_case(q));
        quotes
            .iter()
            .find(|q| {
                if q.starts_with('\'') {
                    file_cells.contains(&q.as_str())
                        && !targets
                            .iter()
                            .any(|t| objects::cells(self.text, self.span(t)).contains(&q.as_str()))
                } else {
                    self.objects.iter().any(|o| has_member(o, q))
                        && !targets.iter().any(|t| match t {
                            Target::Member { object, member, .. } => {
                                self.objects[*object].members[*member].name.eq_ignore_ascii_case(q)
                            }
                            Target::Object(o) | Target::Header(o) => has_member(&self.objects[*o], q),
                        })
                }
            })
            .cloned()
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
            return Err(format!(
                "line {line} is past the end of the file ({} lines)",
                self.starts.len()
            ));
        }
        let line_of = |at: usize| self.starts.partition_point(|&s| s <= at);
        let Some(o) = self
            .objects
            .iter()
            .position(|o| line_of(o.span.start) <= line && line <= line_of(o.span.end.saturating_sub(1)))
        else {
            return match self.objects.first() {
                Some(first) if line < line_of(first.span.start) => self
                    .main_object()
                    .map(Target::Header)
                    .ok_or_else(|| "the file has no object".into()),
                Some(_) => Err(format!("line {line} is outside every object")),
                None => Err("the file has no object".into()),
            };
        };
        // A member's span runs from its lead-in (attributes, comments) to its body's `end;` (`objects::objects`).
        let member = self.objects[o]
            .members
            .iter()
            .position(|m| line_of(m.span.start) <= line && line <= line_of(m.span.end.saturating_sub(1)));
        if let Some(member) = member {
            return Ok(Target::Member {
                object: o,
                member,
                cell: None,
            });
        }
        Ok(Target::Object(o))
    }

    /// The cell of line `line`: its one observation whose first argument is a string literal (`objects::cells`).
    fn cell(&self, line: usize) -> Option<String> {
        let start = self.starts[line - 1];
        let end = self.starts.get(line).copied().unwrap_or(self.text.len());
        match objects::cells(self.text, start..end).as_slice() {
            [one] if one.len() > 2 => Some(one.to_string()),
            _ => None,
        }
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
        assert_eq!(
            t(&[(7, 20)]),
            Ok(vec![member(0, None), member(1, None), member(2, None)])
        );
        assert_eq!(
            t(&[(9, 9), (20, 20)]),
            Ok(vec![member(0, Some("'one'")), member(2, None)])
        );
    }

    #[test]
    fn a_line_past_the_end_or_between_objects_is_an_error() {
        assert!(t(&[(99, 99)]).unwrap_err().contains("past the end"));
        assert!(t(&[(23, 23)]).unwrap_err().contains("is blank"));
        assert!(t(&[(25, 25)]).unwrap_err().contains("lone brace"));
        // A test codeunit's line outside its tests (`Subtype = Test;`); a table's is its object.
        assert!(
            t(&[(4, 4)])
                .unwrap_err()
                .contains("outside every member of a test codeunit")
        );
    }

    #[test]
    fn a_quote_the_file_has_outside_the_cited_member_is_found() {
        let f = File::new(SRC);
        let first = [member(0, None)];
        let q = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(f.quoted_elsewhere(&first, &q(&["'one'", "First"])), None);
        // A cell of another test, or another member's name, read in the citing text.
        assert_eq!(
            f.quoted_elsewhere(&[member(2, None)], &q(&["'one'"])),
            Some("'one'".into())
        );
        assert_eq!(f.quoted_elsewhere(&first, &q(&["helper"])), Some("helper".into()));
        // Text the file does not have is no evidence either way.
        assert_eq!(f.quoted_elsewhere(&first, &q(&["'absent'", "Absent"])), None);
        // An object target holds its members.
        assert_eq!(f.quoted_elsewhere(&[Target::Object(0)], &q(&["Helper", "'two'"])), None);
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

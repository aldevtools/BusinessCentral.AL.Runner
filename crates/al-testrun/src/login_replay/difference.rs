//! What differs between the states of two sessions after the login ([`LoginState`]), and the one text both report lines
//! print for it (`login replay check: ... first: ...` and `login image: ...`): where it differs, never what a row, a
//! session state line or a value holds. The rows are the company's data (a serve request runs on a customer's
//! database), so a value never reaches the report (issue 8211).
//!
//! The text, per kind of [`KINDS`]:
//! - `not comparable: codeunit 5 global 1, SQL counters`: per value with no comparable form, the path its producer gives
//!   apart from why (`al_vm::NotComparable`), never why;
//! - `row 1 (table object 15): field 3 Blob: written_base, content_edits; field 4`: the first row that differs, by its place
//!   in the committed rows and its table's object index; per field (its place in the row, from 1) that differs, and for a
//!   Blob the fields of its buffer (`BlobBuf::differing_fields`); `missing` (the second state has no such row), `added`
//!   (only the second has it), `another table`, `another company`;
//! - `session state 3`: the first session state line that differs, by its place (`missing` or `added` as for a row); the
//!   lines are not `name: value`, so nothing of a line is shown;
//! - `assumption marks`, `open writes`.

use super::LoginState;
use al_builtins::stream::BlobBuf;
use al_builtins::value::Value;
use std::collections::BTreeSet;

/// The kinds of difference between two sessions' states, in the order [`differences`] looks at them: a value with no
/// comparable form (which no replay could give back), the committed rows, the session state lines, the assumption marks,
/// what the company open noted for the tests.
pub(crate) const KINDS: [&str; 5] = ["not comparable", "rows", "session state", "marks", "open writes"];

/// A committed row, each field apart, so a difference names the field: a Blob as its buffer, every other value by its
/// debug form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredRow {
    company: String,
    /// The table's object index.
    table: u32,
    cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Cell {
    Value(String),
    Blob(BlobBuf),
}

impl StoredRow {
    /// Whether a field that is not a Blob holds the value whose debug form is `debug`.
    #[cfg(test)]
    pub(crate) fn holds(&self, debug: &str) -> bool {
        self.cells.iter().any(|c| matches!(c, Cell::Value(v) if v == debug))
    }
}

/// The rows of `tables`, in their order: company, table object and the row with its system fields.
pub(crate) fn stored_rows(tables: &[al_data::memory::ExportedTable]) -> Vec<StoredRow> {
    let cell = |v: &Value| match v {
        Value::Blob(b) => Cell::Blob(b.borrow().clone()),
        v => Cell::Value(format!("{v:?}")),
    };
    (tables.iter())
        .flat_map(|t| {
            (t.rows.iter()).map(|(_, row)| StoredRow {
                company: t.company.clone(),
                table: t.object,
                cells: row.iter().map(cell).collect(),
            })
        })
        .collect()
}

/// What differs between `a` and `b`, the states of two sessions of one key: per kind of [`KINDS`] that differs, its index
/// and where it first differs (see the module comment). One kind does not hide another.
pub(crate) fn differences(a: &LoginState, b: &LoginState) -> Vec<(usize, String)> {
    let opaque: BTreeSet<&str> = (a.state.opaque.iter().chain(&b.state.opaque))
        .map(|v| v.path.as_str())
        .collect();
    let opaque = (!opaque.is_empty()).then(|| format!("not comparable: {}", Vec::from_iter(opaque).join(", ")));
    [
        opaque,
        first_apart(&a.rows, &b.rows).map(|n| row(n, a.rows.get(n), b.rows.get(n))),
        first_apart(&a.state.lines, &b.state.lines).map(|n| {
            let place = presence(a.state.lines.get(n), b.state.lines.get(n));
            format!("session state {}{place}", n + 1)
        }),
        (a.hits != b.hits).then(|| "assumption marks".to_string()),
        (a.open != b.open).then(|| "open writes".to_string()),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(k, d)| Some((k, d?)))
    .collect()
}

/// The place of the first item in which `x` and `y` differ, one of them ending first included.
fn first_apart<T: PartialEq>(x: &[T], y: &[T]) -> Option<usize> {
    let n = (x.iter().zip(y))
        .position(|(p, q)| p != q)
        .unwrap_or(x.len().min(y.len()));
    (n < x.len().max(y.len())).then_some(n)
}

/// `: missing` when only the first state has the item, `: added` when only the second has it.
fn presence<T>(x: Option<&T>, y: Option<&T>) -> &'static str {
    match (x, y) {
        (Some(_), None) => ": missing",
        (None, Some(_)) => ": added",
        _ => "",
    }
}

/// Row `n` of the two states, which differ.
fn row(n: usize, x: Option<&StoredRow>, y: Option<&StoredRow>) -> String {
    let at = n + 1;
    match (x, y) {
        (Some(x), Some(y)) if x.table != y.table => {
            format!("row {at} (table object {} | {}): another table", x.table, y.table)
        }
        (Some(x), Some(y)) if x.company != y.company => format!("row {at} (table object {}): another company", x.table),
        (Some(x), Some(y)) => format!("row {at} (table object {}): {}", x.table, fields(x, y)),
        (Some(r), None) | (None, Some(r)) => format!("row {at} (table object {}){}", r.table, presence(x, y)),
        (None, None) => format!("row {at}"),
    }
}

/// The fields in which two rows of one table differ, by place (from 1); a Blob with the fields of its buffer.
fn fields(x: &StoredRow, y: &StoredRow) -> String {
    let apart: Vec<String> = (0..x.cells.len().max(y.cells.len()))
        .filter_map(|i| match (x.cells.get(i), y.cells.get(i)) {
            (Some(Cell::Blob(p)), Some(Cell::Blob(q))) if p != q => {
                Some(format!("field {} Blob: {}", i + 1, p.differing_fields(q).join(", ")))
            }
            (p, q) if p != q => Some(format!("field {}", i + 1)),
            _ => None,
        })
        .collect();
    apart.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(table: u32, cells: Vec<Cell>) -> StoredRow {
        StoredRow {
            company: "C".into(),
            table,
            cells,
        }
    }

    // A Blob whose buffer differs in two fields while its bytes are equal is named by those fields.
    #[test]
    fn a_row_names_its_fields_and_a_blob_the_fields_of_its_buffer() {
        let blob = al_builtins::stream::new_blob().borrow().clone();
        let mut edited = blob.clone();
        edited.written_base = Some(Vec::new());
        edited.content_edits = 1;
        let x = stored(
            15,
            vec![
                Cell::Value("Integer(1)".into()),
                Cell::Blob(edited),
                Cell::Value("a".into()),
            ],
        );
        let y = stored(
            15,
            vec![
                Cell::Value("Integer(1)".into()),
                Cell::Blob(blob),
                Cell::Value("b".into()),
            ],
        );
        assert_eq!(
            row(0, Some(&x), Some(&y)),
            "row 1 (table object 15): field 2 Blob: written_base, content_edits; field 3"
        );
        assert_eq!(row(1, Some(&x), None), "row 2 (table object 15): missing");
        assert_eq!(row(1, None, Some(&y)), "row 2 (table object 15): added");
        assert_eq!(
            row(0, Some(&x), Some(&stored(18, Vec::new()))),
            "row 1 (table object 15 | 18): another table"
        );
    }

    // The maintainer's review of this check (issue 8211): a value with no comparable form is named by the path its
    // producer gives apart from why (`al_vm::NotComparable`), never by cutting a line, so no text of why reaches the
    // report, whatever it holds.
    #[test]
    fn a_value_with_no_comparable_form_is_named_by_its_path_only() {
        let entry = |path: &str, why: &str| al_vm::NotComparable {
            path: path.into(),
            why: why.into(),
        };
        let mut a = super::super::tests::empty_state();
        a.state.opaque = vec![
            entry("codeunit 5 global 1", "SECRET: x"),
            entry("SQL counters", "SECRET"),
        ];
        let b = super::super::tests::empty_state();
        assert_eq!(
            differences(&a, &b),
            [(0, "not comparable: SQL counters, codeunit 5 global 1".to_string())]
        );
    }
}

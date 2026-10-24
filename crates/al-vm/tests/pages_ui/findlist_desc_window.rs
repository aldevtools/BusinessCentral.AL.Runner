//! A List with OnFindRecord (and OnNextRecord), a descending filtered SourceTableView and no row trigger, over a view of one row
//! and over more rows than a window (AL Runner issues 5738 and 5739, the reporter's page shapes). Pinned by
//! misc/lt-probes-needed-6/PnF06FindListMoves.al:1014-1260 (writer RK, corpus bump 62): RK_DescKind_Find_<N>Rows and
//! RK_DescKind_FindNext_<N>Rows.

use al_testkit::text;

const OBJECTS: &str = r#"
enum 69990 "DW Kind" { Extensible = false; value(0; A) { } value(1; B) { } }
table 69990 "DW Row" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "DW Kind") { } } keys { key(PK; Id) { } } }
codeunit 69990 "DW Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(Entry: Text) begin Buf += Entry + ';'; end;
    procedure Take() Result: Text begin Result := Buf; Buf := ''; end;
}
codeunit 69991 "DW Seed"
{
    procedure Seed(Rows: Integer)
    var Row: Record "DW Row"; Log: Codeunit "DW Log"; I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to Rows do begin
            Row.Init(); Row.Id := I;
            if I mod 2 = 0 then Row.Kind := Row.Kind::B else Row.Kind := Row.Kind::A;
            Row.Insert();
        end;
        Log.Take();
    end;
}
"#;

/// The page `DW <name>` with OnFindRecord (and OnNextRecord when `next`) over `view`, and a codeunit `DW Moves` whose `Run`
/// makes the moves of RK's tests (open, First, Next to the end, Last, Previous to the end, Close) and logs what the page was asked.
fn page_and_moves(view: &str, next: bool) -> String {
    let next_trigger = if next {
        r#"trigger OnNextRecord(Steps: Integer): Integer
  var Log: Codeunit "DW Log"; Moved: Integer;
  begin Moved := Rec.Next(Steps); Log.Add('N(' + Format(Steps) + ')=' + Format(Moved) + ':' + Format(Rec.Id)); exit(Moved); end;"#
    } else {
        ""
    };
    format!(
        r#"page 69990 "DW Page" {{ PageType = List; SourceTable = "DW Row"; SourceTableView = {view}; Editable = false;
  layout {{ area(Content) {{ repeater(R) {{ field(Id; Rec.Id) {{ }} }} }} }}
  trigger OnFindRecord(Which: Text): Boolean
  var Log: Codeunit "DW Log"; Found: Boolean;
  begin Found := Rec.Find(Which); Log.Add('F(' + Which + ')=' + Format(Found) + ':' + Format(Rec.Id)); exit(Found); end;
  {next_trigger} }}
codeunit 69992 "DW Moves"
{{
    local procedure Step(Entries: Text): Text
    begin
        if Entries = '' then exit('');
        exit('[' + Entries + ']');
    end;
    procedure Run(): Text
    var Log: Codeunit "DW Log"; P: TestPage "DW Page"; R: Text; Walk: Text;
    begin
        P.OpenView();
        R := 'open ' + P.Id.Value() + '|' + Log.Take();
        R += ' first ' + Format(P.First()) + '|' + P.Id.Value() + '|' + Log.Take();
        Walk := '';
        while P.Next() do Walk += P.Id.Value() + Step(Log.Take()) + ',';
        R += ' walk ' + Walk + 'end:' + P.Id.Value() + Step(Log.Take());
        R += ' last ' + Format(P.Last()) + '|' + P.Id.Value() + '|' + Log.Take();
        Walk := '';
        while P.Previous() do Walk += P.Id.Value() + Step(Log.Take()) + ',';
        R += ' back ' + Walk + 'end:' + P.Id.Value() + Step(Log.Take());
        P.Close();
        exit(R + ' close ' + Log.Take());
    end;
}}
"#
    )
}

const DESC_KIND: &str = "sorting(Id) order(descending) where(Kind = filter(A))";
const DESC: &str = "sorting(Id) order(descending)";
const ASC: &str = "sorting(Id) order(ascending)";

/// Seeds `seed` rows and runs the moves on a page over `view`.
fn run(view: &str, next: bool, seed: u32) -> String {
    let src = format!(
        "{OBJECTS}\n{}\ncodeunit 69999 T\n{{\n    procedure F(): Text\n    var S: Codeunit \"DW Seed\"; M: Codeunit \"DW Moves\";\n    begin\n        S.Seed({seed});\n        exit(M.Run());\n    end;\n}}\n",
        page_and_moves(view, next)
    );
    text(al_testkit::run_in(&src, "T", "F"))
}

/// The descending filtered page ("Find" or "FindNext") over a view of `rows` rows (2 * `rows` seeded, the odd Ids in the view).
fn moves(which: &str, rows: u32) -> String {
    run(DESC_KIND, which == "FindNext", rows * 2)
}

/// The view's Ids from the top down (desc) or from 1 up (asc), each followed by a comma.
fn ids(rows: u32, desc: bool, skip_first: bool) -> String {
    let mut v: Vec<u32> = (0..rows).map(|i| 2 * i + 1).collect();
    if desc {
        v.reverse();
    }
    v.into_iter()
        .skip(usize::from(skip_first))
        .map(|i| format!("{i},"))
        .collect()
}

/// `N(-1)=-1:<id>;` for each odd id from `from` to `to`.
fn steps_up(from: u32, to: u32) -> String {
    (from..=to).step_by(2).map(|i| format!("N(-1)=-1:{i};")).collect()
}

/// Every move of the walk asks the page nothing; `first` and `open` differ by shape.
fn expect(open: &str, first: &str, rows: u32) -> String {
    let top = 2 * rows - 1;
    format!(
        "open 1|{open} first Yes|{top}|{first} walk {}end:1 last Yes|1| back {}end:{top} close ",
        ids(rows, true, true),
        ids(rows, false, true)
    )
}

/// RK_DescKind_Find_1Rows (PnF06FindListMoves.al:1072), AL Runner issue 5738.
#[test]
fn a_descending_find_list_of_one_row_opens_and_moves_without_asking_the_page() {
    assert_eq!(moves("Find", 1), expect("F(=><)=Yes:1;", "", 1));
}

/// RK_DescKind_FindNext_1Rows (PnF06FindListMoves.al:1185).
#[test]
fn a_descending_find_next_list_of_one_row_steps_back_once_at_open() {
    assert_eq!(moves("FindNext", 1), expect("F(=><)=Yes:1;N(1)=0:1;N(-1)=0:1;", "", 1));
}

/// RK_DescKind_Find_20Rows and RK_DescKind_FindNext_20Rows: Last on the row the list opened on asks nothing.
#[test]
fn a_descending_find_list_of_one_full_window_asks_nothing_on_last() {
    assert_eq!(moves("Find", 20), expect("F(=><)=Yes:1;", "", 20));
    let open = format!("F(=><)=Yes:1;N(1)=0:1;{}N(-1)=0:39;", steps_up(3, 39));
    assert_eq!(moves("FindNext", 20), expect(&open, "", 20));
}

/// RK_DescKind_Find_21Rows, _25Rows (PnF06FindListMoves.al:1137, the reporter's case of AL Runner issue 5739).
#[test]
fn a_descending_find_list_longer_than_a_window_walks_every_row_without_asking_the_page() {
    assert_eq!(moves("Find", 21), expect("F(=><)=Yes:1;", "", 21));
    assert_eq!(moves("Find", 25), expect("F(=><)=Yes:1;", "", 25));
}

/// RK_DescKind_FindNext_21Rows, _25Rows: the open reads a window of 20 rows above the row it opened on and stops; First reads the
/// rest with OnNextRecord(-1) until it answers 0.
#[test]
fn a_descending_find_next_list_longer_than_a_window_reads_the_rest_on_first() {
    let open = format!("F(=><)=Yes:1;N(1)=0:1;{}", steps_up(3, 41));
    assert_eq!(moves("FindNext", 21), expect(&open, "N(-1)=0:41;", 21));
    let first = format!("{}N(-1)=0:49;", steps_up(43, 49));
    assert_eq!(moves("FindNext", 25), expect(&open, &first, 25));
}

/// The moves over a view whose rows in client order are `view` (First lands on `view[0]`), when no move after First asks the page.
fn expect_view(view: &[u32], open: &str, first: &str) -> String {
    let (top, bottom) = (view[0], view[view.len() - 1]);
    let walk: String = view[1..].iter().map(|i| format!("{i},")).collect();
    let back: String = view[..view.len() - 1].iter().rev().map(|i| format!("{i},")).collect();
    format!(
        "open 1|{open} first Yes|{top}|{first} walk {walk}end:{bottom} last Yes|{bottom}| back {back}end:{top} close "
    )
}

/// `N(-1)=-1:<id>;` for each id from `from` to `to`.
fn steps_up_by_one(from: u32, to: u32) -> String {
    (from..=to).map(|i| format!("N(-1)=-1:{i};")).collect()
}

/// RK_Desc_Find_<N>Rows (PnF06FindListMoves.al:1521-1600): a descending view without a WHERE part moves as the filtered one.
#[test]
fn a_descending_find_list_without_a_where_part_moves_without_asking_the_page() {
    for n in [1, 5, 25] {
        let view: Vec<u32> = (1..=n).rev().collect();
        assert_eq!(run(DESC, false, n), expect_view(&view, "F(=><)=Yes:1;", ""), "{n} rows");
    }
}

/// RK_Desc_FindNext_<N>Rows (PnF06FindListMoves.al:1633-1710).
#[test]
fn a_descending_find_next_list_without_a_where_part_steps_up_at_open() {
    assert_eq!(
        run(DESC, true, 1),
        expect_view(&[1], "F(=><)=Yes:1;N(1)=0:1;N(-1)=0:1;", "")
    );
    let view: Vec<u32> = (1..=5).rev().collect();
    let open = format!("F(=><)=Yes:1;N(1)=0:1;{}N(-1)=0:5;", steps_up_by_one(2, 5));
    assert_eq!(run(DESC, true, 5), expect_view(&view, &open, ""));
    let view: Vec<u32> = (1..=25).rev().collect();
    let open = format!("F(=><)=Yes:1;N(1)=0:1;{}", steps_up_by_one(2, 21));
    let first = format!("{}N(-1)=0:25;", steps_up_by_one(22, 25));
    assert_eq!(run(DESC, true, 25), expect_view(&view, &open, &first));
}

/// RK_Asc_Find_<N>Rows (PnF06FindListMoves.al:1745-1822) and RK_Asc_FindNext_1Rows, _5Rows (:1857, :1873).
#[test]
fn an_ascending_find_list_without_a_where_part_moves_without_asking_the_page() {
    for n in [1, 5, 25] {
        let view: Vec<u32> = (1..=n).collect();
        assert_eq!(run(ASC, false, n), expect_view(&view, "F(=><)=Yes:1;", ""), "{n} rows");
    }
    assert_eq!(
        run(ASC, true, 1),
        expect_view(&[1], "F(=><)=Yes:1;N(1)=0:1;N(-1)=0:1;", "")
    );
    assert_eq!(
        run(ASC, true, 5),
        expect_view(
            &[1, 2, 3, 4, 5],
            "F(=><)=Yes:1;N(1)=1:2;N(1)=1:3;N(1)=1:4;N(1)=1:5;N(1)=0:5;N(-1)=0:1;",
            ""
        )
    );
}

/// RK_Asc_FindNext_20Rows, _21Rows, _25Rows (PnF06FindListMoves.al:1889-1934): the open reads a full window; the Next that moves onto
/// its last row reads on with OnNextRecord(1) until that answers 0, and no other move asks the page.
#[test]
fn an_ascending_find_next_list_of_a_full_window_reads_on_when_next_reaches_its_last_row() {
    let open: String = format!(
        "F(=><)=Yes:1;{}N(-1)=0:1;",
        (2..=20).map(|i| format!("N(1)=1:{i};")).collect::<String>()
    );
    for (n, ahead) in [
        (20u32, "N(1)=0:20;".to_string()),
        (21, "N(1)=1:21;N(1)=0:21;".to_string()),
        (
            25,
            format!(
                "{}N(1)=0:25;",
                (21..=25).map(|i| format!("N(1)=1:{i};")).collect::<String>()
            ),
        ),
    ] {
        let walk: String = (2..=n)
            .map(|i| {
                if i == 20 {
                    format!("20[{ahead}],")
                } else {
                    format!("{i},")
                }
            })
            .collect();
        let back: String = (1..n).rev().map(|i| format!("{i},")).collect();
        let want = format!("open 1|{open} first Yes|1| walk {walk}end:{n} last Yes|{n}| back {back}end:1 close ");
        assert_eq!(run(ASC, true, n), want, "{n} rows");
    }
}

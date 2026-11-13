//! DEV ONLY (deleted with the old predicates): the old decision tree of `report_placeholder` over the fact vector, compared
//! with the table on every consistent vector.

use super::Format;
use super::cell::{Answer, ErrorId, Events, Renders};
use super::cells::CELLS;
use super::facts::*;
use super::lookup::{self, Before};

#[derive(Debug, PartialEq, Eq, Clone)]
enum O {
    False(ErrorId, Events),
    Refuse,
    Dataset,
    Discard,
    Run(Renders, Events),
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum O2 {
    Write(Renders, bool),
    Empty,
    FalseKeep,
    TrueNothing,
    Refuse,
}

const UNS: &[&str] = &["substitutereport", "ondocumentready", "onafterdocumentready"];

fn writes(k: Kind, f: Format) -> bool {
    use Format::*;
    matches!(
        (k, f),
        (Kind::Rdlc, Pdf | Word | Excel) | (Kind::Word, Pdf | Word | Html) | (Kind::Excel, Excel)
    )
}

fn sel_ok(s: Selection, f: Format) -> bool {
    matches!(s, Selection::None | Selection::SameKind)
        || (s == Selection::OtherKind && matches!(f, Format::Pdf | Format::Word))
}

fn place(x: &Facts) -> O {
    if x.processing_only || x.file == LayoutFile::Missing || !writes(x.kind, x.format) {
        return if x.listed { O::Discard } else { O::Refuse };
    }
    if x.selection == Selection::TempWord {
        return O::Run(Renders::Selected, Events::AtFact);
    }
    if sel_ok(x.selection, x.format) {
        O::Run(Renders::Default, Events::NotRaised(&[]))
    } else {
        O::Refuse
    }
}

fn old_before(x: &Facts) -> O {
    use Format::*;
    let used = x.answer == AnswerForm::Used;
    if x.answer == AnswerForm::None {
        return place(x);
    }
    if x.file == LayoutFile::Missing && used {
        return O::False(ErrorId::NoValidLayout, Events::NotChecked);
    }
    if x.custom {
        return O::Dataset;
    }
    if used
        && x.target == Target::Stream
        && x.kind == Kind::Word
        && x.decl == Decl::Rendering
        && !x.processing_only
        && x.extensions == Extensions::None
        && x.stored != Stored::None
    {
        let ev = Events::NotRaised(&[]);
        return match (x.stored, x.format) {
            (Stored::CustomUnknownCode, _) => O::False(ErrorId::NoLayout, ev),
            (_, Html) => O::False(ErrorId::UnsupportedFormat, ev),
            _ => O::Run(Renders::StoredRdlc, ev),
        };
    }
    let rdlc_html = x.format == Html && x.layout == Layout::Rdlc;
    let fr = !x.processing_only
        && (rdlc_html || (x.kind == Kind::Word && x.format == Excel) || (x.kind == Kind::Excel && x.format != Excel));
    if fr {
        if !used {
            return O::Refuse;
        }
        if rdlc_html {
            return if sel_ok(x.selection, Html) {
                O::False(ErrorId::UnsupportedFormat, Events::NotChecked)
            } else {
                O::Refuse
            };
        }
        if x.target == Target::File {
            return O::Refuse;
        }
        return if sel_ok(x.selection, x.format) {
            O::False(ErrorId::UnsupportedFormat, Events::NotRaised(UNS))
        } else {
            O::Refuse
        };
    }
    if x.format == Pdf && x.company_build && x.target == Target::File {
        return O::Discard;
    }
    if x.format == Pdf && x.processing_only {
        return O::Dataset;
    }
    place(x)
}

fn old_after(x: &Facts, renders: Renders) -> O2 {
    use Format::*;
    let named = x.named.unwrap();
    if x.rows == Some(Rows::Zero) && x.answer != AnswerForm::None {
        match renders {
            Renders::StoredRdlc | Renders::Selected => return O2::Refuse,
            Renders::Default => {
                let base = !named
                    && x.target == Target::Stream
                    && x.data_items == DataItems::One
                    && x.extensions == Extensions::None;
                let r = x.decl == Decl::Rendering;
                let none = x.triggers == Triggers::None;
                if x.kind == Kind::Word && r && matches!(x.format, Pdf | Word | Html) && base {
                    return if x.triggers == Triggers::OnPost {
                        O2::Refuse
                    } else {
                        O2::Empty
                    };
                }
                if x.kind == Kind::Word && !r && matches!(x.format, Html | Pdf) && base && none {
                    return O2::Empty;
                }
                if x.kind == Kind::Word && !r && x.format == Html && base && x.answer == AnswerForm::Used {
                    return O2::FalseKeep;
                }
                if x.kind == Kind::Rdlc && r && matches!(x.format, Pdf | Word) && base && none {
                    return O2::TrueNothing;
                }
                if x.kind == Kind::Word {
                    return O2::Refuse;
                }
            }
        }
    }
    let rgba = x.format == Html && x.rows == Some(Rows::Some) && {
        let shape = (x.data_items, x.extensions, x.decl);
        let clean = !named && x.dataset_rgba == Some(false);
        let default_word = renders == Renders::Default && x.kind == Kind::Word;
        let base = (DataItems::One, Extensions::None, Decl::Rendering);
        clean
            && ((default_word
                && [
                    base,
                    (DataItems::One, Extensions::None, Decl::Legacy),
                    (DataItems::ParentAndChild, Extensions::None, Decl::Rendering),
                    (DataItems::One, Extensions::OneAddingColumns, Decl::Rendering),
                ]
                .contains(&shape))
                || (renders == Renders::Selected && shape == base))
    };
    O2::Write(renders, rgba)
}

fn new_before(x: &Facts) -> (O, Vec<Events>) {
    match lookup::before_run(&CELLS, x) {
        Before::Answer(r) => (
            match r.answer {
                Answer::False(e) => O::False(e, r.events),
                Answer::Refuse(_) => O::Refuse,
                Answer::Dataset => O::Dataset,
                Answer::Discard => O::Discard,
                _ => unreachable!(),
            },
            vec![],
        ),
        Before::Run(all) => {
            let mut ev: Vec<Events> = all.iter().map(|r| r.events).collect();
            ev.dedup();
            // Renders: of the rows with rows = Some.
            let r = all
                .iter()
                .rev()
                .find_map(|r| match r.answer {
                    Answer::Placeholder(p) => Some(p),
                    _ => None,
                })
                .unwrap();
            (O::Run(r, ev[0]), ev)
        }
        Before::Miss(_) => {
            if x.listed && !lookup::knows(&CELLS, x) {
                (O::Discard, vec![])
            } else {
                (O::Refuse, vec![])
            }
        }
    }
}

fn new_after(x: &Facts) -> O2 {
    match lookup::after_run(&CELLS, x) {
        Ok(r) => match r.answer {
            Answer::Placeholder(p) => O2::Write(p, r.rgba && x.format == Format::Html),
            Answer::Empty => O2::Empty,
            Answer::FalseKeepError => O2::FalseKeep,
            Answer::TrueNothing => O2::TrueNothing,
            Answer::Refuse(_) => O2::Refuse,
            other => panic!("{other:?}"),
        },
        Err(_) => O2::Refuse,
    }
}

#[test]
#[ignore]
fn report_save_table_equals_the_old_predicates() {
    use Format::*;
    let mut checked = 0u64;
    let mut runs = 0u64;
    for format in [Pdf, Word, Excel, Html] {
        for target in [Target::Stream, Target::File] {
            for answer in [AnswerForm::Used, AnswerForm::Statement, AnswerForm::None] {
                if answer == AnswerForm::None && (target != Target::File || !matches!(format, Pdf | Excel)) {
                    continue;
                }
                for kind in [Kind::Rdlc, Kind::Word, Kind::Excel, Kind::Other] {
                    for layout in [Layout::Rdlc, Layout::NotRdlc] {
                        if layout == Layout::Rdlc && kind != Kind::Rdlc {
                            continue;
                        }
                        for decl in [Decl::Rendering, Decl::Legacy] {
                            for file in [LayoutFile::Present, LayoutFile::Missing] {
                                for processing_only in [false, true] {
                                    for extensions in
                                        [Extensions::None, Extensions::OneAddingColumns, Extensions::Other]
                                    {
                                        for custom in [false, true] {
                                            if custom && answer == AnswerForm::None {
                                                continue;
                                            }
                                            for company_build in [false, true] {
                                                for listed in [false, true] {
                                                    for selection in [
                                                        Selection::None,
                                                        Selection::SameKind,
                                                        Selection::OtherKind,
                                                        Selection::Unknown,
                                                        Selection::DesignTimeHeld,
                                                        Selection::TempWord,
                                                    ] {
                                                        let entry = format == Html
                                                            && target == Target::Stream
                                                            && kind == Kind::Word
                                                            && extensions == Extensions::None
                                                            && !processing_only
                                                            && file == LayoutFile::Present
                                                            && !custom;
                                                        if (selection == Selection::TempWord)
                                                            != (entry && selection == Selection::TempWord)
                                                            || (selection == Selection::DesignTimeHeld && entry)
                                                        {
                                                            continue;
                                                        }
                                                        for stored in [
                                                            Stored::None,
                                                            Stored::RdlcBuiltin,
                                                            Stored::CustomUnknownCode,
                                                        ] {
                                                            let ok = match stored {
                                                                Stored::None => true,
                                                                Stored::RdlcBuiltin => {
                                                                    selection
                                                                        == if kind == Kind::Rdlc {
                                                                            Selection::SameKind
                                                                        } else {
                                                                            Selection::OtherKind
                                                                        }
                                                                }
                                                                Stored::CustomUnknownCode => {
                                                                    selection == Selection::Unknown
                                                                }
                                                            };
                                                            if !ok {
                                                                continue;
                                                            }
                                                            let x = Facts {
                                                                format,
                                                                target,
                                                                answer,
                                                                kind,
                                                                layout,
                                                                decl,
                                                                file,
                                                                processing_only,
                                                                data_items: DataItems::One,
                                                                extensions,
                                                                triggers: Triggers::None,
                                                                custom,
                                                                company_build,
                                                                listed,
                                                                selection,
                                                                stored,
                                                                named: None,
                                                                rows: None,
                                                                dataset_rgba: None,
                                                            };
                                                            checked += 1;
                                                            let old = old_before(&x);
                                                            let (new, evs) = new_before(&x);
                                                            assert_eq!(old, new, "before run: {x:?}");
                                                            assert!(
                                                                evs.len() <= 1,
                                                                "events differ among candidates: {x:?} {evs:?}"
                                                            );
                                                            if let O::Run(renders, _) = old {
                                                                runs += 1;
                                                                after_all(&x, renders);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("checked {checked} vectors, {runs} runs");
}

fn after_all(x: &Facts, renders: Renders) {
    let named_options: &[bool] = match (x.selection, x.stored) {
        (_, Stored::RdlcBuiltin | Stored::CustomUnknownCode) => &[true],
        (Selection::SameKind | Selection::OtherKind | Selection::Unknown, _) => &[true],
        (Selection::TempWord, _) => &[false],
        _ => &[false, true],
    };
    for rows in [Rows::Zero, Rows::Some] {
        for data_items in [DataItems::One, DataItems::ParentAndChild, DataItems::Other] {
            for triggers in [Triggers::None, Triggers::WithoutOnPost, Triggers::OnPost] {
                for &named in named_options {
                    for rgba in [false, true] {
                        let y = Facts {
                            rows: Some(rows),
                            data_items,
                            triggers,
                            named: Some(named),
                            dataset_rgba: Some(rgba),
                            ..*x
                        };
                        assert_eq!(old_after(&y, renders), new_after(&y), "after run: {y:?}");
                    }
                }
            }
        }
    }
}

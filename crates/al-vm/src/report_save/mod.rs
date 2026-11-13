//! The answer of a rendering save as one table of cells (ADR-040 amendment 2026-11-13, part 1).
//!
//! A save as Pdf, Word, Excel or Html computes one fact vector ([`facts::Facts`]; each group has one owner: the call here, the
//! report `fact_report`, the layout selection `fact_selection`, the run `fact_run`) and looks it up in the table (`cells`,
//! `lookup`). The row says whether the save answers false at once, runs and writes a placeholder, answers after a run that wrote
//! no dataset row, or refuses; which layout events a visible subscriber refuses; and whether the rgba replace of `Email
//! Message.Create` is measured on the Html it writes. No row: the save refuses and names the facts no row has. A save no row
//! of the table is about at all keeps the ADR-040 allow-list's answer (run, write nothing).
//!
//! Step 1 round 1 moves today's cells into the table without changing an answer; the writing of the output stays in
//! `report_placeholder`.

mod cell;
mod cells;
mod errors;
mod events;
mod fact_report;
mod fact_run;
mod fact_selection;
mod facts;
mod lookup;

#[cfg(test)]
mod tests;

use std::sync::Arc;

pub(crate) use crate::report_placeholder::Format;
use crate::report_placeholder::SaveOutput;
use crate::reports::{SaveTarget, Schema};
use crate::{Session, Signal};
use al_builtins::native::AlError;
pub(crate) use cell::Renders;
use cell::{Answer, Cell, Events};
pub(crate) use facts::AnswerForm;
use facts::{Facts, Selection, Target};

/// A save that runs: its facts, to look up again after the run.
#[derive(Clone, Debug)]
pub(crate) struct Plan {
    facts: Facts,
    /// The layout a temporary selection named.
    selected: Option<Arc<str>>,
}

impl Plan {
    pub(crate) fn format(&self) -> Format {
        self.facts.format
    }
}

/// What a save does before the run.
pub(crate) enum BeforeRun {
    /// Run the report, then [`Session::save_after_run`].
    Run(Plan),
    /// Run and write the Xml dataset.
    Dataset,
    /// Run and write nothing.
    Discard,
    /// Answer false with this error in `GetLastErrorText`; nothing runs and nothing is written.
    False(AlError),
}

/// What a save that ran writes or answers.
pub(crate) enum AfterRun {
    /// Write the placeholder of the format, rendered by this layout; `rgba` the row's rgba column.
    Write {
        renders: Renders,
        selected: Option<Arc<str>>,
        rgba: al_builtins::rendered_text::RgbaReplace,
    },
    /// The answer, with nothing written (the last error is set).
    Answer(bool),
}

fn what(facts: &Facts, schema: &Schema) -> String {
    format!(
        "Report.SaveAs as {} of report \"{}\"",
        facts.format.name(),
        schema.name()
    )
}

/// The save's target as a fact.
fn target_fact(target: &SaveTarget) -> Target {
    match target {
        SaveTarget::Stream(_) => Target::Stream,
        SaveTarget::File(_) => Target::File,
    }
}

impl Session {
    /// The facts of a save of `schema` as `format` to `target`, taken as `answer`, before the run. `custom`: a layout of Type
    /// Custom renders.
    fn save_facts(
        &mut self,
        schema: &Schema,
        format: Format,
        target: &SaveTarget,
        answer: AnswerForm,
        custom: bool,
    ) -> Result<(Facts, Option<Arc<str>>), Signal> {
        let r = fact_report::report_facts(&self.engine.program, schema);
        let target = target_fact(target);
        let what = format!("Report.SaveAs as {}", format.name());
        let entry = fact_selection::TempEntry {
            format,
            target,
            kind: r.kind,
            extensions: r.extensions,
            processing_only: r.processing_only,
            file: r.file,
            custom,
            what: &what,
        };
        let (selection, selected) = self.selection_fact(schema, Some(&entry))?;
        let facts = Facts {
            format,
            target,
            answer,
            kind: r.kind,
            layout: r.layout,
            decl: r.decl,
            file: r.file,
            processing_only: r.processing_only,
            data_items: r.data_items,
            extensions: r.extensions,
            triggers: r.triggers,
            custom,
            company_build: self.company_build,
            listed: self.output_is_ignored(schema),
            selection,
            stored: self.stored_fact(schema)?,
            named: None,
            rows: None,
            dataset_rgba: None,
        };
        Ok((facts, selected))
    }

    /// What a save as `format` (not Xml) does before the report runs.
    pub(crate) fn save_before_run(
        &mut self,
        schema: &Schema,
        format: Format,
        target: &SaveTarget,
        answer: AnswerForm,
        custom: bool,
    ) -> Result<BeforeRun, Signal> {
        let (facts, selected) = self.save_facts(schema, format, target, answer, custom)?;
        let table: &[Cell] = &cells::CELLS;
        match lookup::before_run(table, &facts) {
            lookup::Before::Answer(row) => {
                self.check_events(schema, &facts, row.events)?;
                mark(row);
                match row.answer {
                    Answer::False(e) => Ok(BeforeRun::False(errors::error_of(
                        e,
                        self.object_id_of(schema.object as u32),
                        format,
                    ))),
                    Answer::Refuse(why) => Err(Signal::unsupported(format!("{}, {why}", what(&facts, schema)))),
                    Answer::Dataset => Ok(BeforeRun::Dataset),
                    Answer::Discard => Ok(BeforeRun::Discard),
                    _ => unreachable!("a row that answers before the run"),
                }
            }
            lookup::Before::Run(all) => {
                for row in &all {
                    self.check_events(schema, &facts, row.events)?;
                }
                // When every row the run can end in marks the same assumption (a stored selection, a temporary
                // selection), it is marked now, as main did; otherwise the row the run ends in marks it.
                if let Some(a) = lookup::one_mark_after_any_run(table, &facts) {
                    al_builtins::assumptions::assume(a);
                }
                Ok(BeforeRun::Run(Plan { facts, selected }))
            }
            lookup::Before::Miss(miss) => {
                // A save the table has no row about keeps the ADR-040 allow-list's answer (`Session::output_is_ignored`).
                if facts.listed && !lookup::knows(table, &facts) {
                    return Ok(BeforeRun::Discard);
                }
                Err(Signal::unsupported(miss.reason(&what(&facts, schema))))
            }
        }
    }

    /// What a `Report.SaveAs` (static or on a variable) as `format` (not Xml) writes once it runs, or the error of a false
    /// answer without a run.
    pub(crate) fn static_or_variable_save(
        &mut self,
        schema: &Schema,
        format: &str,
        target: &SaveTarget,
        result_used: bool,
        custom: bool,
    ) -> Result<Result<SaveOutput, AlError>, Signal> {
        let Some(format) = Format::from_name(format) else {
            return Err(Signal::unsupported(format!(
                "Report.SaveAs of \"{}\" as {format}",
                schema.name()
            )));
        };
        let answer = if result_used {
            AnswerForm::Used
        } else {
            AnswerForm::Statement
        };
        Ok(Ok(
            match self.save_before_run(schema, format, target, answer, custom)? {
                BeforeRun::Run(plan) => SaveOutput::Rendered(Box::new(plan)),
                BeforeRun::Dataset => SaveOutput::Dataset,
                BeforeRun::Discard => SaveOutput::Discard,
                BeforeRun::False(e) => return Ok(Err(e)),
            },
        ))
    }

    /// What a save that ran writes or answers: the row of its facts with the run's (`rows` dataset rows, the Xml `dataset`).
    pub(crate) fn save_after_run(
        &mut self,
        plan: &Plan,
        schema: &Schema,
        dataset: &[u8],
        rows: usize,
    ) -> Result<AfterRun, Signal> {
        let facts = self.after_run_facts(&plan.facts, schema, dataset, rows)?;
        let row = lookup::after_run(&cells::CELLS, &facts)
            .map_err(|miss| Signal::unsupported(miss.reason(&what(&facts, schema))))?;
        mark(row);
        let used = facts.answer == AnswerForm::Used;
        let empty = || errors::error_of(cell::ErrorId::ResultEmpty, 0, facts.format);
        match row.answer {
            Answer::Placeholder(renders) => Ok(AfterRun::Write {
                renders,
                selected: plan.selected.clone(),
                rgba: if row.rgba {
                    al_builtins::rendered_text::RgbaReplace::Measured
                } else {
                    al_builtins::rendered_text::RgbaReplace::NotMeasured(lookup::rgba_reason(&cells::CELLS, &facts))
                },
            }),
            Answer::Empty if used => {
                self.last_error = empty();
                Ok(AfterRun::Answer(false))
            }
            Answer::Empty => Err(Signal::Error(Box::new(empty()))),
            Answer::FalseKeepError => Ok(AfterRun::Answer(false)),
            Answer::TrueNothing => Ok(AfterRun::Answer(true)),
            Answer::Refuse(why) => Err(Signal::unsupported(format!("{}, {why}", what(&facts, schema)))),
            Answer::False(_) | Answer::Dataset | Answer::Discard => unreachable!("rows read no run fact"),
        }
    }

    fn check_events(&self, schema: &Schema, facts: &Facts, events: Events) -> Result<(), Signal> {
        match events {
            Events::NotRaised(except) => self.refuse_layout_event_subscribers(
                schema,
                &format!("Report.SaveAs as {}", facts.format.name()),
                except,
                None,
            ),
            Events::NotChecked | Events::AtFact => Ok(()),
        }
    }

    /// Whether a static SaveAs as `format` is one the default layout cannot give, so it raises SubstituteReport and nothing
    /// after it (reports/layouts-saveas/AMRdlcOutput.al:TextRdlc_SaveAsHtml_IsRefused `SubstituteReport(79256,SaveAs,req=Yes);`;
    /// AMReportOutputTests.al:221 and :257 `SubstituteReport(7925N,SaveAs,req=Yes);`): a row of the table answers it with
    /// `ReportUnsupportedFormat` without reading the selection.
    pub(crate) fn format_unsupported_by_layout(&self, schema: &Schema, format: &str) -> bool {
        let Some(format) = Format::from_name(format) else {
            return false;
        };
        let r = fact_report::report_facts(&self.engine.program, schema);
        cells::CELLS.iter().any(|c| {
            use cell::K::{Is, Unread};
            c.answer == Answer::False(cell::ErrorId::UnsupportedFormat)
                && c.key.stored == Unread
                && c.key.format == Is(format)
                && (c.key.kind == Unread || c.key.kind == Is(r.kind))
                && (c.key.layout == Unread || c.key.layout == Is(r.layout))
                && c.key.processing_only == Is(r.processing_only)
        })
    }

    /// Refuses a Run, RunModal or Execute of a report that renders while a subscriber the test can see listens to a layout
    /// event other than SubstituteReport, or while a layout selection may pick another layout than an RDLC or Word layout of
    /// the report (`report_placeholder::run`; it joins the table in round 3).
    pub(crate) fn refuse_run_layout_events(&mut self, schema: &Schema) -> Result<(), Signal> {
        self.refuse_layout_event_subscribers(schema, "a run", &["substitutereport"], None)?;
        let (selection, _) = self.selection_fact(schema, None)?;
        if matches!(selection, Selection::None | Selection::SameKind | Selection::OtherKind) {
            return Ok(());
        }
        Err(Signal::unsupported(format!(
            "a run of report \"{}\" while the layout selection is {} (BC may render another layout, and an Excel one writes \
             no output; this runtime answers only for the report's own layout)",
            schema.name(),
            facts::Describe::describe(&selection)
        )))
    }
}

fn mark(row: &Cell) {
    if let Some(a) = row.evidence.marks() {
        al_builtins::assumptions::assume(a);
    }
}

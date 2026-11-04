//! `Filter Helper Triggers.MakeDateFilter`: the platform asks subscribers to repair a Date filter text it cannot read.
//!
//! Pinned by events/platform-triggers/PtObserve.al (`Filter_Date`, `Bad_Date`), against PtFixtures.al (the subscriber)
//! and PtObjects.al (`datefiltertext`, `baddate`):
//! - The event fires once for each piece of the text that does not read as a date, with that piece (`311220` of
//!   `010120..311220`), before any error is raised.
//! - If the subscriber changes the piece and the new text does not read either, the event fires again with the new text.
//!   When a call leaves the text as it was, the piece has failed.
//! - A text with a `..` whose piece failed then fires once more with the whole text, by the same rule.
//!   A text without `..` fires only for the piece (`baddate` m1: one call).
//! - A repair that makes every piece read gives the filter built from the repaired pieces (`01/01/20..12/31/20`);
//!   a failure raises the error about the original text, whatever the subscribers did (`err-same`).
//!
//! Not pinned: a text with other filter syntax (`|`, `&`, `<`, `>`, `=`, `@`, quotes, wildcards, `...`), a session that is not
//! English, placeholders. The event is not raised for those (as before this change; a probe is wanted). Refused when it is raised:
//! more than one failing piece, a subscriber that changes the text a third time in a row.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::Value;

fn unpinned<T>(why: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(format!("MakeDateFilter: {why} (not pinned)")))
}

impl Session {
    /// One call of the event `event` (`makedatefilter`, ...): the text the subscribers leave in `param`.
    fn make_filter_event(&mut self, event: &'static str, param: &str, text: &str) -> Result<String, Signal> {
        let names = [param.to_string()];
        let mut args = [Value::Text(text.to_string().into())];
        let call = crate::dispatch::EventCall {
            object: crate::events::FILTER_HELPER_TRIGGERS,
            event,
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        // A subscriber of a dependency app other than Microsoft's Filter Tokens has an effect no case pins: refuse.
        let engine = self.engine.clone();
        if let Some(subs) = engine
            .events
            .subscribers
            .get(&(crate::events::FILTER_HELPER_TRIGGERS, event.to_string()))
        {
            for s in subs {
                let o = &engine.program.objects[s.object];
                let dependency = engine.program.apps.get(o.app as usize).is_some_and(|a| a.dependency);
                if dependency && !o.name.eq_ignore_ascii_case("Filter Tokens Impl.") {
                    return unpinned(&format!(
                        "the subscriber {} \"{}\" of a dependency app",
                        o.kind.label(),
                        o.name
                    ));
                }
            }
        }
        // Microsoft's own subscriber (System Application "Filter Tokens") is not run: it reads date formulas, which are not
        // pinned, and running it turned a passing corpus case into a refusal (record/filters/
        // DateFilterTextForms.al:GetRange_DayAboveTwelve_English). So only the subscribers of the app under test run.
        self.events.skip_dependency_subscribers = true;
        let raised = self.raise(&call, &mut args);
        self.events.skip_dependency_subscribers = false;
        raised?;
        Ok(al_builtins::native::text_of(&args[0]).unwrap_or_default().to_string())
    }

    /// Asks the subscribers about `piece` until it reads or a call leaves it alone. `Ok(None)`: it does not read.
    fn repair_piece(
        &mut self,
        event: &'static str,
        param: &str,
        piece: &str,
        reads: &dyn Fn(&str) -> bool,
    ) -> Result<Option<String>, Signal> {
        let mut current = piece.to_string();
        for round in 0..3 {
            let next = self.make_filter_event(event, param, &current)?;
            if next == current {
                return Ok(None);
            }
            if reads(&next) {
                return Ok(Some(next));
            }
            if round == 2 {
                return unpinned("a subscriber changed an unreadable text three times in a row");
            }
            current = next;
        }
        Ok(None)
    }

    /// The text to parse instead of `text`, after the subscribers had their say, or `None` when the text stays unreadable.
    /// `reads` says whether a Date filter text parses.
    pub(crate) fn repair_date_filter_text(
        &mut self,
        text: &str,
        reads: &dyn Fn(&str) -> bool,
    ) -> Result<Option<String>, Signal> {
        let t = text.trim();
        // Only the inputs the corpus pins raise the event: a single word of letters (`abc`), or a range of two six-digit
        // numbers (`010120..311220`). In both, Microsoft's Filter Tokens subscriber leaves the text alone (PtObserve.al:157,
        // Bad_Date), which is why it is not run. Any other text did not raise the event before and still does not.
        let letters = !t.is_empty() && t.chars().all(|c| c.is_ascii_alphabetic());
        let range = t.split_once("..").is_some_and(|(a, b)| {
            [a, b]
                .iter()
                .all(|p| p.len() == 6 && p.bytes().all(|c| c.is_ascii_digit()))
        });
        if !letters && !range {
            return Ok(None);
        }
        if t.contains("...") || t.chars().any(|c| "|&<>=@*?'\"()%".contains(c)) || t.matches("..").count() > 1 {
            // Not the pinned shape: the event is not raised for it (as before this change).
            return Ok(None);
        }
        let pieces: Vec<&str> = t.split("..").collect();
        let mut fixed: Vec<String> = pieces.iter().map(|p| p.trim().to_string()).collect();
        let mut failed = 0;
        for (i, p) in pieces.iter().enumerate() {
            let p = p.trim();
            if p.is_empty() || reads(p) {
                continue;
            }
            failed += 1;
            if failed > 1 {
                return unpinned("two unreadable pieces in one Date text");
            }
            if let Some(new) = self.repair_piece("makedatefilter", "DateFilterText", p, reads)? {
                fixed[i] = new;
            }
        }
        let rebuilt = fixed.join("..");
        if reads(&rebuilt) {
            return Ok(Some(rebuilt));
        }
        if pieces.len() > 1 {
            // The whole text is asked about as well (Filter_Date m1, m2).
            return self.repair_piece("makedatefilter", "DateFilterText", t, reads);
        }
        Ok(None)
    }

    /// `MakeIntFilter` and `MakeTimeFilter`: a filter text of one word of letters (`abc`) that does not read is asked about as
    /// `MakeDateFilter` asks about a piece: once (m1), again with a changed text that does not read (m2), and the text a
    /// subscriber repairs is parsed instead (m3) (events/platform-triggers/PtObserve.al:Bad_Int, Bad_Time).
    ///
    /// A range or two alternatives (`1..x`, `x|1`, `abc..xyz`) ask about the piece that does not read, by the same rule, and
    /// only about it: the whole text is not asked about after a failure (unlike `MakeDateFilter`). A repaired piece goes back
    /// into the text, and the next piece that does not read is asked about in turn. `MakeIntFilter` is also asked about `1.5`
    /// and `1 2` (events/table-events/BdFilterEvents.al:BdFilter_Int_1 to _4 and _8, BdFilter_Time_1 and _2).
    /// Whether BC raises the event for another text that does not evaluate (`*`, a word with hyphens, other filter syntax) is
    /// not pinned: with a subscriber present it is refused rather than answered with the evaluate error (docs/probes-needed.md).
    pub(crate) fn repair_word_filter_text(
        &mut self,
        event: &'static str,
        param: &str,
        text: &str,
        failing: &str,
        parse: &dyn Fn(&str) -> Result<(), String>,
    ) -> Result<Option<String>, Signal> {
        let t = text.trim();
        let refuse = || {
            Err(Signal::Unsupported(format!(
                "SetFilter of `{t}`, which does not evaluate, with {} subscribers (whether BC raises the event for it is not pinned)",
                event_name(event)
            )))
        };
        let pinned_piece = |p: &str| {
            al_builtins::filter::letters_only(p)
                || (event == "makeintfilter"
                    && (p.split_once('.').is_some_and(|(a, b)| {
                        [a, b]
                            .iter()
                            .all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit()))
                    }) || al_builtins::filter::spaced_number(p)))
        };
        if t.contains("...") || t.chars().any(|c| "&<>=@*?'\"()%".contains(c)) {
            return refuse();
        }
        let sep = match (t.matches("..").count(), t.matches('|').count()) {
            (0, 0) => "",
            (1, 0) => "..",
            (0, 1) => "|",
            _ => return refuse(),
        };
        let mut pieces: Vec<String> = if sep.is_empty() {
            vec![t.to_string()]
        } else {
            t.split(sep).map(|p| p.trim().to_string()).collect()
        };
        let mut failing = failing.to_string();
        let mut repaired = false;
        for _ in 0..pieces.len() {
            let Some(i) = pieces.iter().position(|p| *p == failing) else {
                return refuse();
            };
            if !pinned_piece(&failing) {
                return refuse();
            }
            let reads = |p: &str| parse(p).is_ok();
            let Some(new) = self.repair_piece(event, param, &failing, &reads)? else {
                // A failure after an earlier piece was repaired: which text BC's error names is not pinned.
                if repaired {
                    return refuse();
                }
                return Ok(None);
            };
            repaired = true;
            pieces[i] = new;
            let rebuilt = pieces.join(sep);
            match parse(&rebuilt) {
                Ok(()) => return Ok(Some(rebuilt)),
                Err(e) => match al_builtins::filter::not_evaluated_value(&e) {
                    Some(next) => failing = next.to_string(),
                    None => return refuse(),
                },
            }
        }
        refuse()
    }
}

/// The event's name as AL code spells it.
fn event_name(event: &str) -> &'static str {
    match event {
        "makeintfilter" => "MakeIntFilter",
        "maketimefilter" => "MakeTimeFilter",
        _ => "Filter Helper Triggers",
    }
}

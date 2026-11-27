//! What a passing test leaves of a number sequence under BC's TestRunnerHub with RequiredTestIsolation = Function
//! (al-testrun `isolation::after_test`, al-runner issue 9250).
//!
//! The log of the NumberSequence calls that change a sequence (Insert, Next, Range, Restart, Delete) and of the
//! transaction ends, from the start of a test codeunit (before its OnRun) to its end (`Session::clear_unmeasured`), and
//! the decision per sequence a test changed: put back to the checkpoint after OnRun, kept as the test left it, or refused
//! for the later tests. Only the cells of corpus PR 1580 answer, through the assumption `hub-function-sequence`; every
//! other shape refuses the later calls on the sequence. Entry points: `begin_hub_sequence_log`,
//! `hub_test_sequences`, `settle_hub_sequences`.

use crate::Session;
use crate::sequence_start::CommitBy;
use al_builtins::assumptions::Assumption;
use std::collections::HashSet;

type Key = (String, String);

/// One entry of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    /// A NumberSequence call that changes the sequence (its method name).
    Call(&'static str, Key),
    /// A commit, by this.
    Commit(CommitBy),
    /// A rollback.
    Rollback,
    /// A test method body begins (`true`) or ends; what the runner does around it is outside the test.
    Body(bool),
}

/// The log of a test codeunit under the hub's Function isolation ([`Session::begin_hub_sequence_log`]).
#[derive(Debug, Default)]
pub(crate) struct Log {
    events: Vec<Event>,
    /// The sequences when the codeunit began, before its OnRun.
    at_begin: HashSet<Key>,
    /// The length of `events` at the checkpoint after OnRun (`Session::storage_checkpoint`).
    checkpoint: usize,
    /// The length of `events` when the running test began.
    test_from: usize,
}

/// What the end of a passing or failed test does with a sequence it changed ([`Session::hub_test_sequences`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubSequence {
    /// Back to its state at the checkpoint after OnRun.
    PutBack,
    /// As the test left it.
    Keep,
    /// A later test's call on it is refused.
    Refuse,
}

const NEXT: &str = "Next";

impl Session {
    /// A test codeunit under BC's TestRunnerHub with RequiredTestIsolation = Function begins (before its OnRun): its
    /// NumberSequence changes and transaction ends are logged until the codeunit ends.
    pub fn begin_hub_sequence_log(&mut self) {
        let at_begin = self.sequences.keys().cloned().collect();
        self.unmeasured.hub_log = Some(Log {
            at_begin,
            ..Log::default()
        });
    }

    /// The checkpoint after OnRun is taken: the tests begin here.
    pub(crate) fn hub_log_checkpoint(&mut self) {
        if let Some(log) = &mut self.unmeasured.hub_log {
            log.checkpoint = log.events.len();
            log.test_from = log.events.len();
        }
    }

    /// Logs a NumberSequence call `method` on `key` that changes the sequence.
    pub(crate) fn hub_log_call(&mut self, method: &'static str, key: &Key) {
        if let (Some(log), "Insert" | "Next" | "Range" | "Restart" | "Delete") = (&mut self.unmeasured.hub_log, method)
        {
            log.events.push(Event::Call(method, key.clone()));
        }
    }

    /// A test method body begins (`start`) or ends.
    pub fn hub_log_body(&mut self, start: bool) {
        if let Some(log) = &mut self.unmeasured.hub_log {
            log.events.push(Event::Body(start));
        }
    }

    /// Logs a transaction end (`commit`: by `by`).
    pub(crate) fn hub_log_tx_end(&mut self, commit: bool, by: CommitBy) {
        if let Some(log) = &mut self.unmeasured.hub_log {
            log.events.push(match commit {
                true => Event::Commit(by),
                false => Event::Rollback,
            });
        }
    }

    /// The sequences the running test changed, each with what its end does with it (call before the test's rollback).
    /// Corpus PR 1580, tests/al-language/codeunits/run-onrun/SeqNextHubFunction.al, BC's TestRunnerHub with
    /// RequiredTestIsolation = Function, a sequence missing when the codeunit began and inserted by its OnRun, the first
    /// test of the codeunit that changes it, passing:
    /// (the cells pass their `CompanyWide` as the CompanySpecific argument):
    /// - put back: OnRun took Next twice, the test only restarted it (E4 :407) or only deleted it (E5 :499), for all
    ///   companies;
    /// - kept: OnRun took Next twice, the test took Next once, per company (E2 :234), or for all companies with an AL
    ///   `Commit()` after it (E3 :320); OnRun took no Next, the test took Next once, for all companies (O1) or per
    ///   company (O2).
    ///
    /// Everything else refuses, E1 too (for all companies, OnRun took Next twice, the test took Next once without a
    /// commit: BC reads the sequence missing, observed only, :151-174).
    pub fn hub_test_sequences(&self, failed: bool) -> Vec<(Key, HubSequence)> {
        let Some(log) = &self.unmeasured.hub_log else {
            return Vec::new();
        };

        // The test's own events: those of its body (a call outside it, in a runner trigger, is refused below).
        let window = &log.events[log.test_from..];
        let body = match (
            window.iter().position(|e| *e == Event::Body(true)),
            window.iter().position(|e| *e == Event::Body(false)),
        ) {
            (Some(from), Some(to)) if from < to => &window[from + 1..to],
            _ => &[][..],
        };
        let mut keys: Vec<&Key> = (window.iter())
            .filter_map(|e| match e {
                Event::Call(_, k) => Some(k),
                _ => None,
            })
            .collect();
        keys.sort();
        keys.dedup();
        let decide = |key: &Key| -> HubSequence {
            let calls = |from: usize, to: usize| -> Vec<&'static str> {
                (log.events[from..to].iter())
                    .filter_map(|e| match e {
                        Event::Call(m, k) if k == key => Some(*m),
                        _ => None,
                    })
                    .collect()
            };
            let test: Vec<&Event> = (body.iter())
                .filter(|e| match e {
                    Event::Call(_, k) => k == key,
                    _ => true,
                })
                .collect();
            let on_run = calls(0, log.checkpoint);
            let earlier = calls(log.checkpoint, log.test_from);
            let per_company = key.0 == *self.cx.company;
            let all_companies = key.0.is_empty();
            let next_commit = [Event::Call(NEXT, key.clone()), Event::Commit(CommitBy::Statement)];
            let in_body = (window.iter())
                .filter(|e| matches!(e, Event::Call(_, k) if k == key))
                .count()
                == (body.iter())
                    .filter(|e| matches!(e, Event::Call(_, k) if k == key))
                    .count();
            let first = in_body && !failed && !log.at_begin.contains(key) && earlier.is_empty();
            let prepared = on_run == ["Insert", NEXT, NEXT];
            let fresh = on_run == ["Insert"];
            let only = |m: &'static str| test == [&Event::Call(m, key.clone())];
            match () {
                _ if first && prepared && all_companies && (only("Restart") || only("Delete")) => HubSequence::PutBack,
                _ if first && prepared && per_company && only(NEXT) => HubSequence::Keep,
                _ if first && prepared && all_companies && test.iter().copied().eq(next_commit.iter()) => {
                    HubSequence::Keep
                }
                _ if first && fresh && (per_company || all_companies) && only(NEXT) => HubSequence::Keep,
                _ => HubSequence::Refuse,
            }
        };
        keys.into_iter().map(|k| (k.clone(), decide(k))).collect()
    }

    /// After the test's rollback: puts back, keeps or refuses each sequence of [`Session::hub_test_sequences`] (`at`: the
    /// checkpoint after OnRun; `why`: the refusal text of a key). A put back or kept sequence runs on the assumption
    /// `hub-function-sequence` for every later call of the codeunit on it. The next test begins here.
    pub fn settle_hub_sequences(
        &mut self,
        decided: Vec<(Key, HubSequence)>,
        at: &crate::StorageCheckpoint,
        why: impl Fn(&Key) -> String,
    ) {
        for (key, what) in decided {
            match what {
                HubSequence::PutBack => {
                    match at.sequence(&key) {
                        Some(s) => self.sequences.insert(key.clone(), s.clone()),
                        None => self.sequences.remove(&key),
                    };
                    self.assume_sequence(&key, Assumption::HubFunctionSequence);
                }
                HubSequence::Keep => self.assume_sequence(&key, Assumption::HubFunctionSequence),
                HubSequence::Refuse => self.refuse_unmeasured_sequence(&key, why(&key)),
            }
        }
        if let Some(log) = &mut self.unmeasured.hub_log {
            log.test_from = log.events.len();
        }
    }
}

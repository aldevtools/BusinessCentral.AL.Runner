//! Event dispatch: publisher calls reach subscribers; manual subscriptions
//! are bound to codeunit instances (`BindSubscription`).
//!
//! Semantics, with the corpus tests that pin them:
//! - Subscriber parameters are matched to the publisher's by name; a
//!   subscriber may declare any subset (codeunit/TestCodeunitEventSenderPosition.al:
//!   a trailing parameter may be omitted), and with `IncludeSender` the
//!   one parameter that matches no publisher parameter is the sender, at any
//!   position (same file, SenderFirst/Middle/Last).
//! - `var` parameters write back to the publisher (codeunit/TestEventContracts.al:12,
//!   record/TestTableEventByRefMutation.al:23).
//! - An error in a subscriber reaches the publisher's caller
//!   (codeunit/TestEventUnderscoreName.al, UnderscoredEvent_SubscriberError_Propagates).
//! - A `Manual` subscriber fires only on instances bound by `BindSubscription`
//!   (codeunit/TestEventManualBinding.al, Contracts 1-3), once per bound
//!   instance (Contract 7), on the bound instance itself (Contract 2). A
//!   binding lasts until `UnbindSubscription` or until the last reference to
//!   the instance is gone (TestEventManualBinding.al Contract 10,
//!   TestEventManualBindingByValue.al); a `SingleInstance` codeunit is its
//!   session-wide instance, so it is never gone.
//! - Isolated events: each subscriber is its own transaction when nothing is
//!   pending (codeunit/TestIsolatedEventSubscriberError.al).

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_builtins::ops;
use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Inst = Rc<RefCell<Instance>>;

/// An event being raised: which one, and what the publisher offers its subscribers.
pub(crate) struct EventCall<'a> {
    /// The object subscribers name (the base table for events of a table extension).
    pub object: usize,
    /// Lower-case event name.
    pub event: &'a str,
    /// Filters field events by field name.
    pub element: Option<&'a str>,
    pub isolated: bool,
    /// The publisher's parameter names, matching `args` of `raise`.
    pub names: &'a [String],
    /// The publishing instance, for `IncludeSender` events.
    pub sender: Option<Value>,
}

/// One manual subscription: a codeunit instance whose manual subscribers receive events.
pub(crate) struct Binding {
    pub instance: Weak<RefCell<Instance>>,
}

/// The error of a failed `BindSubscription` statement, code `EventBinding`.
fn binding_error(id: u32, why: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message: format!("The binding of codeunit {id} was unsuccessful. {why}"),
        code: "EventBinding".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// Runs a publisher routine: no body, only its subscribers. Returns the
    /// parameters as the subscribers left them (for `var` write-back).
    pub(crate) fn publish(
        &mut self,
        object: usize,
        routine: usize,
        instance: Inst,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let origins = std::mem::take(&mut self.events.var_origins);
        let lens = std::mem::take(&mut self.events.publisher_var_lens);
        let engine = self.engine.clone();
        let p = &engine.events.publishers[&(object, routine)];
        self.note_event(p.object, &p.event, None);
        let r = &engine.program.objects[object].routines[routine];
        let mut values = Vec::with_capacity(r.params);
        for (i, a) in args.into_iter().enumerate().take(r.params) {
            values.push(match (a, r.by_var.get(i).copied().unwrap_or(false)) {
                (Value::Record(rc), false) => self.record_by_value(&rc),
                // A Notification handed by value to an event is the caller's own notification: a subscriber's SetData and Message
                // reach the publisher's caller (events/lt-notification-by-value/NotificationByValue.al:Event_ByValueArgument_SubscriberChanges).
                (a @ Value::Library(al_builtins::library_kind::LibraryKind::Notification, _), false) => a,
                // An unknown event argument a subscriber passes on to its own event stays unknown (see `call_inner`).
                (a @ Value::Unknown(_), false) => a,
                (a, _) => ops::convert(a, &r.locals[i].ty)?,
            });
        }
        let mut sender_refused = None;
        let sender = if p.include_sender {
            let o = &engine.program.objects[object];
            match (o.kind, o.rec_slot) {
                // The sender of a reportextension's event is the report variable the extension belongs to: the subscriber
                // calls the report's procedures and the extension's, and sees the state the caller set before the raise
                // (ReportExtensionSender.al:Sender_IsTheRaisingReportVariable, Sender_TwoReportVariables, Sender_FreshReportVariable).
                (al_ir::ObjKind::ReportExtension, _) => {
                    match instance.borrow().base.as_ref().and_then(std::rc::Weak::upgrade) {
                        Some(base) => Some(Value::Codeunit(base)),
                        None => {
                            sender_refused = Some(format!(
                                "{}: IncludeSender on a reportextension that runs without its report (not pinned)",
                                r.name
                            ));
                            None
                        }
                    }
                }
                (al_ir::ObjKind::Codeunit, _) => Some(Value::Codeunit(instance)),
                // The sender of a page's event is the page (a `Page` variable), not its Rec: Item Tracking Lines
                // publishes with IncludeSender and Asm. Item Tracking Mgt. takes `var sender: Page "Item Tracking
                // Lines"`. Pinned by pages/lt-page-event-sender/PageEventSender.al:PageEvent_FromOpenPage_SenderIsTheOpenedPage, PageEvent_FromPageVariable_SenderIsTheVariable.
                (al_ir::ObjKind::Page, _) => match o.globals.iter().position(|g| g.name == "CurrPage") {
                    Some(slot) => Some(instance.borrow().globals[slot].clone()),
                    None => return unsupported(format!("{}: IncludeSender on a page without CurrPage", r.name)),
                },
                // The sender of a report's event is the running report, as a Report variable: the subscriber calls the
                // report's procedures and sees its state (pages/lt-page-report-events/P84ReportSender.al).
                (al_ir::ObjKind::Report, _) => Some(Value::Codeunit(instance)),
                (_, Some(slot)) => Some(instance.borrow().globals[slot].clone()),
                _ => return unsupported(format!("{}: IncludeSender on a {}", r.name, o.kind)),
            }
        } else {
            None
        };
        let names: Vec<String> = r.locals[..r.params].iter().map(|s| s.name.clone()).collect();
        let call = EventCall {
            object: p.object,
            event: &p.event,
            element: None,
            isolated: p.isolated,
            names: &names,
            sender,
        };
        // The publisher's event frame stands below its subscribers' (corpus
        // session/TestSessionInformationCallstackContracts.al:187).
        self.frames.push(object);
        self.call_stack.push(crate::CallFrame {
            event: true,
            routine,
            pos: al_ir::NO_POS,
        });
        self.events.publish_frame = true;
        self.events.sender_refused = sender_refused;
        self.events.var_origins = origins;
        self.events.publisher_var_lens = lens;
        let raised = self.raise(&call, &mut values);
        self.events.sender_refused = None;
        self.events.publish_frame = false;
        let top = self.call_stack.pop();
        self.frames.pop();
        if let Err(e) = raised {
            // What earlier subscribers wrote back is what the publisher's caller sees.
            let mut e = e;
            if let (Signal::Error(err), Some(top)) = (&mut e, top) {
                self.add_frame(err, object, top);
            }
            if matches!(e, Signal::Error(_)) {
                self.error_finals = Some(values);
            }
            return Err(e);
        }
        Ok((Value::Null, values))
    }

    /// Delivers the event to its subscribers. `var` parameters of subscribers are written
    /// back into `args`. Isolated events: see below.
    pub(crate) fn raise(&mut self, ev: &EventCall, args: &mut [Value]) -> Result<(), Signal> {
        let from_publisher = std::mem::take(&mut self.events.publish_frame);
        // A table event (OnAfterInsertEvent and the others the platform raises) puts no frame of its own
        // between the subscriber and the code that raised it (session/TestSessionCallstackSourcesContracts.al:
        // Callstack_FromTablePageReportAndExtensionCode, `table event`); page events and Global Triggers are
        // not measured.
        let unpinned = !from_publisher
            && self
                .engine
                .program
                .objects
                .get(ev.object)
                .is_none_or(|o| !o.is_kind(al_ir::ObjKind::Table));
        if unpinned {
            self.events.unpinned_event_depth += 1;
        }
        let r = self.raise_to_subscribers(ev, args);
        if unpinned {
            self.events.unpinned_event_depth -= 1;
        }
        r
    }

    fn raise_to_subscribers(&mut self, ev: &EventCall, args: &mut [Value]) -> Result<(), Signal> {
        let EventCall {
            object,
            event,
            element,
            isolated,
            names,
            sender,
        } = ev;
        let (object, event, element, isolated) = (*object, *event, *element, *isolated);
        let engine = self.engine.clone();
        let sender_refused = self.events.sender_refused.take();
        let origins = std::mem::take(&mut self.events.var_origins);
        // The declared lengths of the publisher's `var` Text and Code arguments: a subscriber's `var` parameter reports the
        // length of the publisher's argument (text/lt-text-functions/MaxStrLen.Codeunit.al:172-201,
        // PYC_MaxStrLen_EventSubscriberVarParameter: `direct raise field text100` = 100, `subscriber via var param text30` =
        // 30). An event the platform raises has none: MaxStrLen refuses there when it runs.
        let lens = std::mem::take(&mut self.events.publisher_var_lens);
        // The publisher's global a `var` argument names is the variable itself while the event runs: whoever changes it (a subscriber
        // through its parameter, or through the sender) changes what the next subscriber receives and what the publisher's caller sees.
        let refresh = |args: &mut [Value]| {
            for (j, o) in origins.iter().enumerate() {
                if let (Some((inst, g)), Some(a)) = (o, args.get_mut(j)) {
                    *a = inst.borrow().globals[*g].clone();
                }
            }
        };
        for (s, inst) in self.event_targets(object, event, element)? {
            refresh(args);
            let r = &engine.program.objects[s.object].routines[s.routine];
            let mut map: Vec<Option<usize>> = Vec::with_capacity(r.params);
            let mut sender_taken = false;
            for slot in &r.locals[..r.params] {
                if let Some(j) = names.iter().position(|n| n.eq_ignore_ascii_case(&slot.name)) {
                    map.push(Some(j));
                } else if sender.is_some() && !sender_taken {
                    sender_taken = true;
                    map.push(None);
                } else if let Some(why) = &sender_refused {
                    return unsupported(why.clone());
                } else {
                    return unsupported(format!(
                        "subscriber {}.{}: parameter `{}` matches no parameter of event `{event}`",
                        engine.program.objects[s.object].name, r.name, slot.name
                    ));
                }
            }
            let call_args = map
                .iter()
                .map(|m| match m {
                    Some(j) => args[*j].clone(),
                    None => sender.clone().expect("sender checked"),
                })
                .collect();
            // An Isolated event runs each subscriber in its own transaction: with nothing pending
            // in the caller's, a subscriber's error is swallowed and only its writes are rolled
            // back; with a write pending the event is not isolated and the error propagates
            // (codeunit/TestIsolatedEventSubscriberError.al). The swallowed error becomes the last
            // error, and a subscriber that succeeds has its writes committed (corpus
            // record/TestTableEventSequenceContracts.al:IsolatedEvent_SubscriberError_BecomesTheLastError_...).
            *self
                .events
                .event_stats
                .calls
                .borrow_mut()
                .entry((s.object, s.routine))
                .or_insert(0) += 1;
            let isolate = isolated && !self.store.has_uncommitted();
            let before = isolate.then(|| self.store.clone());
            let phase = self.tx.tx_phase;
            // A `var` argument that is a global of the publisher is the same variable as the subscriber's parameter.
            self.events.next_aliases = map
                .iter()
                .enumerate()
                .filter_map(|(i, m)| match (m, r.by_var[i]) {
                    (Some(j), true) => origins.get(*j).and_then(Clone::clone).map(|(inst, g)| (i, inst, g)),
                    _ => None,
                })
                .collect();
            if engine.facts[s.object][s.routine].var_lens {
                self.events.next_var_lens = map
                    .iter()
                    .map(|m| m.and_then(|j| lens.get(j).copied().flatten()))
                    .collect();
            }
            self.events.next_is_subscriber = true;
            let called = self.call(s.object, s.routine, inst, call_args);
            self.events.next_is_subscriber = false;
            self.events.next_aliases.clear();
            self.events.next_var_lens.clear();
            let (_, finals) = match called {
                Err(Signal::Error(e)) if isolate => {
                    self.store = before.expect("snapshot taken when isolating");
                    // The subscriber's own transaction is gone with its writes.
                    self.tx.tx_phase = phase;
                    self.last_error = *e;
                    continue;
                }
                r => r?,
            };
            if isolate {
                self.commit_transaction();
            }
            for (i, m) in map.iter().enumerate() {
                if let (Some(j), true) = (m, r.by_var[i]) {
                    args[*j] = finals[i].clone();
                    if let Some(Some((inst, g))) = origins.get(*j) {
                        inst.borrow_mut().globals[*g] = finals[i].clone();
                    }
                }
            }
        }
        refresh(args);
        Ok(())
    }

    /// The subscribers an event reaches right now, each with the instance it
    /// runs on: static subscribers always, manual ones once per bound instance.
    /// `element` filters field events by field name.
    fn event_targets(
        &mut self,
        object: usize,
        event: &str,
        element: Option<&str>,
    ) -> Result<Vec<(crate::events::Subscriber, Inst)>, Signal> {
        let engine = self.engine.clone();
        // A subscriber BC would run and this program has no code for: running the event without it
        // would be a wrong answer (ADR-013 decision 6, hard rule 4).
        if let Some(m) = engine.events.missing_for(object, event, element) {
            return unsupported(format!(
                "event {event}: {} subscribes to it and was not compiled (no code the workspace reaches uses it)",
                m.subscriber
            ));
        }
        let Some(subs) = engine.events.subscribers.get(&(object, event.to_string())) else {
            return Ok(Vec::new());
        };
        let mut targets = Vec::new();
        for s in subs {
            if self
                .events
                .not_installed_apps
                .as_ref()
                .is_some_and(|apps| apps.contains(&engine.program.objects[s.object].app))
            {
                continue;
            }
            if object == engine.events.global_key {
                if let Some(u) = engine.events.unavailable_global.get(&(s.object, s.routine)) {
                    // A subscriber that cannot run would receive this event. BC runs it, and what it
                    // does (change log, Dataverse, API webhooks) AL code can see: refuse. The setup
                    // events are the one exception: while no table the subscriber's decision reads
                    // holds a row, nothing is configured and it answers "no" for every table.
                    let setup = matches!(event, "getglobaltabletriggermask" | "getdatabasetabletriggersetup");
                    let configured = u.config.iter().find(|&&t| self.store.has_rows(t));
                    if !setup || configured.is_some() {
                        let o = &engine.program.objects[s.object];
                        return unsupported(format!(
                            "Global Triggers event {event}: subscriber {}.{} cannot run ({}){}",
                            o.name,
                            o.routines[s.routine].name,
                            u.why,
                            configured.map_or(String::new(), |&t| format!(
                                " and {} holds rows, which it reads",
                                engine.program.objects[t as usize].name
                            ))
                        ));
                    }
                    continue;
                }
            }
            if let Some(el) = element {
                if !s.element.eq_ignore_ascii_case(el) {
                    continue;
                }
            }
            if self.events.skip_dependency_subscribers
                && engine
                    .program
                    .apps
                    .get(engine.program.objects[s.object].app as usize)
                    .is_some_and(|a| a.dependency)
            {
                continue;
            }
            if s.manual {
                for inst in self.bound_instances(s.object) {
                    targets.push((s.clone(), inst));
                }
            } else {
                targets.push((s.clone(), self.static_instance(s.object)?));
            }
        }
        // Inside one app, subscribers run in ascending codeunit id, a bound manual one among them (corpus
        // record/TestTableEventSequenceContracts.al:Subscribers_RunInAscendingCodeunitIdOrder_...;
        // events/lt-bw-subscriber-order/BwSubscriberOrderTests.al:66-67 for five codeunits whose names, namespaces and
        // files predict other orders); the sort is stable, so one codeunit's subscribers keep their declaration order.
        let program = &engine.program;
        let id_of = |s: &crate::events::Subscriber| program.objects[s.object].id.unwrap_or(0);
        let app_of = |s: &crate::events::Subscriber| program.objects[s.object].app as usize;
        if targets.windows(2).all(|w| app_of(&w[0].0) == app_of(&w[1].0)) {
            targets.sort_by_key(|(s, _)| id_of(s));
            return Ok(targets);
        }
        // Across apps, app by app in dependency order (BwSubscriberOrderTests.al:52-56: `A1;A2;B1;B2;C1;C2;P;Q;R;S;T;` on
        // BC 27.5, 28.5 and 29.0, where A, B, C and the main app depend on each other in that order), also where an app
        // has lower subscriber codeunit ids than an app it depends on (BwSubscriberOrderTests.al:150: `A;B1;B2;C;M;`, B
        // and C with lower ids than A). The probes cannot tell dependency order from publish order (CI publishes in
        // dependency order). Apps that do not depend on each other: see `outside` below.
        let mut lowest: Vec<(usize, u32)> = Vec::new();
        for (s, _) in &targets {
            let (app, id) = (app_of(s), id_of(s));
            match lowest.iter_mut().find(|(a, _)| *a == app) {
                Some(l) => l.1 = l.1.min(id),
                None => lowest.push((app, id)),
            }
        }
        lowest.sort_by_key(|&(_, id)| id);
        let depends = |a: usize, b: usize| {
            engine
                .events
                .depends
                .get(a)
                .and_then(|r| r.get(b))
                .copied()
                .unwrap_or(false)
        };
        // Dependency order; apps that do not depend on each other keep the order of their lowest subscriber id.
        let mut order: Vec<usize> = Vec::with_capacity(lowest.len());
        let mut left: Vec<usize> = lowest.iter().map(|&(a, _)| a).collect();
        while !left.is_empty() {
            let at = left
                .iter()
                .position(|&a| !left.iter().any(|&b| b != a && depends(a, b)))
                .unwrap_or(0);
            order.push(left.remove(at));
        }
        let dependency_app = |a: usize| program.apps.get(a).is_some_and(|x| x.dependency);
        let name = |a: usize| {
            program
                .apps
                .get(a)
                .map_or_else(|| format!("app {a}"), |x| x.name.clone())
        };
        // Kept, not pinned: two dependency apps (Microsoft's) that do not depend on each other keep the order the
        // runtime had before the cross-app probe, ascending codeunit id across them. An app under test in such a pair
        // is refused unless it is the one app outside the others' dependencies (below).
        let mut kept = false;
        let mut unrelated: Vec<(usize, usize)> = Vec::new();
        for (i, &first) in order.iter().enumerate() {
            for &later in &order[i + 1..] {
                if depends(later, first) {
                    continue;
                }
                if dependency_app(first) && dependency_app(later) {
                    kept = true;
                    continue;
                }
                unrelated.push((first, later));
            }
        }
        if !unrelated.is_empty() {
            let refuse = |why: String| {
                let (first, later) = unrelated[0];
                unsupported(format!(
                    "event {event}: subscribers in apps {} and {}, whose order BC does not pin for this event \
                     (the corpus pins app by app in dependency order where each app depends on the apps before it, \
                     and one app outside every other app's dependencies after them{why})",
                    name(first),
                    name(later)
                ))
            };
            // One app outside: corpus app D depends on A only, no app depends on it, it is published right after A
            // and before B, C and the main app, and its subscriber runs last, after the main app
            // (BwSubscriberOrderTests.al, last test, SubscriberOrder_AppsThatDoNotDependOnEachOther_FiveApps:
            // `A;B1;B2;C;M;D;` on BC 27.5, 28.5 and 29.0). That one shape does not tell "outside apps last" from a
            // dependency order that breaks ties by the higher app id (as text), the later app name or the app
            // published last: each of them must put the app last too, else the raise is refused.
            let outside = order
                .iter()
                .copied()
                .find(|&x| unrelated.iter().all(|&(f, l)| f == x || l == x));
            let Some(x) = outside else {
                return refuse(String::new());
            };
            let info = |a: usize| program.apps.get(a);
            if kept || dependency_app(x) || order.iter().any(|&a| dependency_app(a)) {
                return refuse(": here a dependency app takes part".into());
            }
            if (0..program.apps.len()).any(|a| a != x && depends(a, x)) {
                return refuse(format!(": another app depends on {}", name(x)));
            }
            if !order.iter().any(|&a| a != x && depends(x, a)) {
                return refuse(format!(": {} depends on none of the other apps", name(x)));
            }
            for &(f, l) in &unrelated {
                let y = if f == x { l } else { f };
                let (Some(xi), Some(yi)) = (info(x), info(y)) else {
                    return refuse(String::new());
                };
                if xi.id.to_ascii_lowercase() <= yi.id.to_ascii_lowercase() {
                    return refuse(format!(": {} has a lower app id than {}", name(x), name(y)));
                }
                if xi.name.to_lowercase() <= yi.name.to_lowercase() {
                    return refuse(format!(": {} has a name before {}", name(x), name(y)));
                }
                // Apps are numbered in publish order (the corpus runner loads them as corpus CI publishes them).
                if x > y {
                    return refuse(format!(": {} is published after {}", name(x), name(y)));
                }
            }
            order.retain(|&a| a != x);
            order.push(x);
        }
        // One block per app in that order; with a kept pair, each run of dependency apps none of which depends on
        // another is one block in codeunit id order.
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        for (i, &app) in order.iter().enumerate() {
            let merge = kept
                && i > 0
                && dependency_app(app)
                && blocks.last().is_some_and(|&(_, b)| {
                    blocks
                        .iter()
                        .filter(|&&(_, ob)| ob == b)
                        .all(|&(o, _)| dependency_app(o) && !depends(app, o))
                });
            let block = match blocks.last() {
                Some(&(_, b)) if merge => b,
                Some(&(_, b)) => b + 1,
                None => 0,
            };
            blocks.push((app, block));
        }
        let rank = |s: &crate::events::Subscriber| {
            blocks
                .iter()
                .find(|&&(a, _)| a == app_of(s))
                .map_or(usize::MAX, |&(_, b)| b)
        };
        targets.sort_by_key(|(s, _)| (rank(s), id_of(s)));
        Ok(targets)
    }

    /// Whether raising `event` of `object` now would reach any subscriber.
    pub(crate) fn has_listeners(&mut self, object: usize, event: &str, element: Option<&str>) -> Result<bool, Signal> {
        if !self.engine.events.is_subscribed(object, event) {
            return Ok(false);
        }
        Ok(!self.event_targets(object, event, element)?.is_empty())
    }

    /// The instance a static (automatic) subscriber runs on: the session's one instance for a
    /// `SingleInstance` codeunit, else a fresh one per event, so its globals start at their defaults
    /// every time (corpus record/TestTableEventSequenceContracts.al:Insert_EventsAndTrigger_WithAndWithoutRunTrigger,
    /// where the After event sees `hits=0` after the Before event counted 1).
    fn static_instance(&mut self, object: usize) -> Result<Inst, Signal> {
        let fresh = Rc::new(RefCell::new(Instance {
            object: object as u32,
            ..Default::default()
        }));
        Ok(self.shared_instance(fresh))
    }

    /// Live bound instances of codeunit `object`, oldest binding first.
    fn bound_instances(&mut self, object: usize) -> Vec<Inst> {
        self.events.bindings.retain(|b| b.instance.strong_count() > 0);
        self.events
            .bindings
            .iter()
            .filter_map(|b| b.instance.upgrade())
            .filter(|i| i.borrow().object as usize == object)
            .collect()
    }

    /// The bound instances per manual subscriber codeunit, for the Event Subscription table.
    fn sync_active_instances(&mut self) {
        self.events.bindings.retain(|b| b.instance.strong_count() > 0);
        let mut active = std::collections::HashMap::new();
        for b in &self.events.bindings {
            if let Some(i) = b.instance.upgrade() {
                *active.entry(i.borrow().object as usize).or_insert(0) += 1;
            }
        }
        *self.events.event_stats.active.borrow_mut() = active;
    }

    /// `BindSubscription(Codeunit)` (`bind`) and `UnbindSubscription(Codeunit)`.
    pub(crate) fn subscription(&mut self, bind: bool, cu: Value, ret_used: bool) -> Result<Value, Signal> {
        let Value::Codeunit(rc) = cu else {
            return unsupported(format!("BindSubscription of {}", cu.describe()));
        };
        let inst = self.shared_instance(rc);
        let object = inst.borrow().object as usize;
        let manual = self.engine.object_facts[object].manual_subscriber;
        self.events.bindings.retain(|b| b.instance.strong_count() > 0);
        let at = self
            .events
            .bindings
            .iter()
            .position(|b| b.instance.upgrade().is_some_and(|i| Rc::ptr_eq(&i, &inst)));
        if bind {
            // Not Manual: false, no error (codeunit/TestEventManualBinding.al Contract 5, the
            // Boolean form only; the statement form is not pinned).
            if !manual {
                return if ret_used {
                    Ok(Value::Boolean(false))
                } else {
                    // corpus record/TestTableEventSequenceContracts.al:BindSubscription_Manual_Twice_And_Static_Errors
                    Err(binding_error(
                        self.engine.program.objects[object].id.unwrap_or(0),
                        "The codeunit does not use manual binding",
                    ))
                };
            }
            if at.is_some() {
                // Boolean form: false (Contract 4); statement form: an error (Contract 8).
                return if ret_used {
                    Ok(Value::Boolean(false))
                } else {
                    Err(binding_error(
                        self.engine.program.objects[object].id.unwrap_or(0),
                        "The codeunit has already been bound.",
                    ))
                };
            }
            self.ensure_initialized(&inst);
            self.events.bindings.push(Binding {
                instance: Rc::downgrade(&inst),
            });
            self.sync_active_instances();
            Ok(Value::Boolean(true))
        } else if manual {
            // True even when it was never bound (Contract 6, "container-verified").
            if let Some(i) = at {
                self.events.bindings.remove(i);
            }
            self.sync_active_instances();
            Ok(Value::Boolean(true))
        } else if ret_used {
            // The corpus pins only that it does not raise (TestEventContracts.al Contract 6).
            Ok(Value::Boolean(false))
        } else {
            Ok(Value::Null)
        }
    }
}

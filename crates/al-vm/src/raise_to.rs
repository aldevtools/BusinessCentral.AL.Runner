//! Raising an event to some of its subscribers only (al-testrun `login.rs`: the login events raised to the apps under
//! test's own subscribers). The event code runs them, so what holds for the event holds for them: an isolated event
//! rolls back a subscriber's error and goes on (codeunits/TestIsolatedEventSubscriberError.al).

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::Instance;

impl Session {
    /// Calls the parameterless publisher routine `routine` of `object`, its event reaching only `subscribers`
    /// (object, routine); the events raised inside them reach every subscriber.
    pub fn raise_to(&mut self, object: usize, routine: usize, subscribers: &[(usize, usize)]) -> Result<(), Signal> {
        let r = &self.engine.program.objects[object].routines[routine];
        if r.params != 0 {
            return Err(Signal::unsupported(format!("event {} declares parameters", r.name)));
        }
        let fresh = al_builtins::instance_cycles::shared(Instance {
            object: object as u32,
            ..Default::default()
        });
        let inst = self.shared_instance(fresh);
        self.events.only_targets = Some(subscribers.to_vec());
        let raised = self.call(object, routine, inst, Vec::new()).map(|_| ());
        self.events.only_targets = None;
        raised
    }
}

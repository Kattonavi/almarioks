//! The clock of the session, injected (plan, "Regla de dependencia"): the time stamped on entries, the name of the
//! recovery file and the 5 s limit of RF-33 come from here, so the tests use a clock they move by hand and never
//! sleep (spec, "Cómo se prueba", row 29, 73, 34).

use std::{
    sync::{Arc, Mutex, PoisonError},
    thread,
    time::{Duration, SystemTime},
};

/// What an alarm runs when its time has passed.
pub type Ring = Box<dyn FnOnce() + Send>;

/// The time and one way to be told that an amount of it has passed.
pub trait Clock: Send + Sync {
    /// The current time.
    fn now(&self) -> SystemTime;

    /// Runs `ring` once, on some other thread, when `after` has passed. There is no way to cancel it: `ring` must be
    /// harmless when it is no longer needed.
    fn alarm(&self, after: Duration, ring: Ring);
}

/// The clock of the machine.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemClock;

fn fire(slot: &Mutex<Option<Ring>>) {
    let ring = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(ring) = ring {
        ring();
    }
}

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn alarm(&self, after: Duration, ring: Ring) {
        let slot = Arc::new(Mutex::new(Some(ring)));
        let for_thread = Arc::clone(&slot);
        let spawned = thread::Builder::new()
            .name("almarioks-alarm".to_owned())
            .spawn(move || {
                thread::sleep(after);
                fire(&for_thread);
            });
        if spawned.is_err() {
            // No thread to wait on: the limit counts as passed, so whoever waits never waits without a bound.
            fire(&slot);
        }
    }
}

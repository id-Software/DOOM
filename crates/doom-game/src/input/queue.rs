//! The event ring of `D_PostEvent` and `D_ProcessEvents` (d_main.c:141-177).

use super::{Event, InputError};

/// `MAXEVENTS` (d_event.h:108).
pub const MAXEVENTS: usize = 64;

/// `events[MAXEVENTS]`, `eventhead` and `eventtail` (d_main.c:141-143), zero at start.
///
/// `D_PostEvent` has no overflow check (d_main.c:150-154): when 64 events are posted with
/// none processed, the head catches up with the tail and the ring reads as empty, so all 64
/// are lost, and later posts overwrite slots from the tail. That is reproduced (the `queue_*`
/// goldens). The app's [`super::EventLayer`] never posts more than
/// [`super::MAX_EVENTS_PER_BUILD`] events per build, so the app never loses one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventQueue {
    events: [Event; MAXEVENTS],
    head: usize,
    tail: usize,
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EventQueue {
    /// An empty ring. The initial slot contents are never read: a slot between the tail and
    /// the head has always been written by a post.
    pub const fn new() -> Self {
        Self {
            events: [Event::KeyUp(0); MAXEVENTS],
            head: 0,
            tail: 0,
        }
    }

    /// `D_PostEvent` (d_main.c:150-154): `events[eventhead] = *ev;
    /// eventhead = (++eventhead)&(MAXEVENTS-1);`.
    pub fn post(&mut self, event: Event) {
        self.events[self.head] = event;
        self.head = (self.head + 1) & (MAXEVENTS - 1);
    }

    /// The number of events the next [`EventQueue::process`] would deliver.
    pub fn pending(&self) -> usize {
        self.head.wrapping_sub(self.tail) & (MAXEVENTS - 1)
    }

    /// `D_ProcessEvents`' walk from the tail to the head (d_main.c:170-176), calling
    /// `respond` on each event in order. The store-demo guard (d_main.c:166-168) applies only
    /// to `commercial` without `map01` and is not modelled. On an error the tail stays on the
    /// refused event, which is not consumed.
    pub fn process(
        &mut self,
        mut respond: impl FnMut(Event) -> Result<(), InputError>,
    ) -> Result<(), InputError> {
        while self.tail != self.head {
            respond(self.events[self.tail])?;
            self.tail = (self.tail + 1) & (MAXEVENTS - 1);
        }
        Ok(())
    }
}

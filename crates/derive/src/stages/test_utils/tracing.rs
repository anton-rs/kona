//! This module contains a subscriber layer for `tracing-subscriber` that collects traces and their
//! log levels.

use alloc::{format, string::String, sync::Arc, vec::Vec};
use spin::Mutex;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

/// The storage for the collected traces.
pub type TraceStorage = Arc<Mutex<Vec<(Level, String)>>>;

#[derive(Debug, Default)]
pub struct CollectingLayer {
    pub storage: TraceStorage,
}

impl CollectingLayer {
    pub fn new(storage: TraceStorage) -> Self {
        Self { storage }
    }
}

impl<S: Subscriber> Layer<S> for CollectingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();
        let message = format!("{:?}", event);

        let mut storage = self.storage.lock();
        storage.push((level, message));
    }
}

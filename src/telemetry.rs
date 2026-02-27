use std::io::{self, Write};

use tokio::{task, task::JoinHandle};
use tracing::{Span, Subscriber, subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(
        name,
        // Wrap the original sink with newline writer to give line space between logs
        MakeNewlineWriter(sink),
    );

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

// `init_subscriber` should only be called once, or it will panic!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Can Panic
    LogTracer::init().expect("Failed to set logger");
    // Can Panic
    subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}

// NewlineWriter: A wrapper that adds \n after every write
//
// MakeNewlineWriter: A factory that produces these wrappers
//
// Usage: Plug it in where you'd normally put your output
pub struct NewlineWriter<W> {
    inner: W,
}

impl<W: Write> Write for NewlineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)?;
        // Add newline after each write
        self.inner.write_all(b"\n")?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub struct MakeNewlineWriter<M>(pub M);

impl<'a, M> MakeWriter<'a> for MakeNewlineWriter<M>
where
    M: MakeWriter<'a> + 'a,
{
    type Writer = NewlineWriter<<M as MakeWriter<'a>>::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        NewlineWriter {
            inner: self.0.make_writer(),
        }
    }
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = Span::current();
    task::spawn_blocking(move || current_span.in_scope(f))
}

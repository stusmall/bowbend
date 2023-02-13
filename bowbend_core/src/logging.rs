use tracing::Level;
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

/// Set up the tracing module.  This dumps out detailed traces of the exact
/// code path to stdout.  This is only useful for internal development and not
/// to a consumer of the library.
pub(crate) fn setup_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_span_events(FmtSpan::FULL)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

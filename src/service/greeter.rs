use tracing::instrument;

/// Greets someone by name.
#[instrument(ret)]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

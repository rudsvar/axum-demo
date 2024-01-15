//! A service for greeting someone.

use tracing::instrument;

/// Returns a greeting based on someone's name.
#[instrument(ret)]
pub fn hello(name: &str) -> String {
    format!("Hello, {name}!")
}

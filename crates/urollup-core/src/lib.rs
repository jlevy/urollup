//! The urollup accounting core.
//!
//! This library will hold everything a usage rollup needs apart from the process
//! boundary: source discovery and capture, the reconciled ledger and its analytical
//! identities, accounting, portable artifacts and the shared query API that the CLI and
//! the optional `serve` feature both call. Its modules follow the design's layers
//! (`docs/urollup-design.md` §1.6); each is an empty skeleton until the milestone that
//! implements it.
//!
//! The crate has no CLI, HTTP or async-runtime dependencies.

// Library code reports failures as values; a panic here would take down a `serve`
// process's request, or turn a malformed log line into a crash (design §8.2).
#![deny(clippy::panic)]

pub mod accounting;
pub mod artifacts;
pub mod ledger;
pub mod query;
pub mod sources;

/// The version of this crate, as declared in the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn version_is_the_workspace_semver() {
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "expected MAJOR.MINOR.PATCH, got {VERSION}");
        assert!(
            parts.iter().all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())),
            "expected numeric semver components, got {VERSION}"
        );
    }
}

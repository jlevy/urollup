//! A byte-substring line prefilter, which is only ever a hint (design §2.2).
//!
//! Adapted from ccusage `rust/crates/ccusage-core/src/fast.rs` at `bd7f89b`; see
//! PROVENANCE.md and THIRD-PARTY-NOTICES.md. Changes: the `SmallVec` storage is a `Vec`,
//! the two modes are one public constructor pair, and the documentation states the rule
//! urollup adds, that a prefilter miss is a counted skip and never a reason to drop a
//! record from coverage.
//!
//! A prefilter answers "could this line matter?" so that a decoder pays for a full JSON
//! parse only on lines that can carry usage. It must therefore never be the last word:
//! [`crate::sources::manifest::RecordCounters::skipped`] counts every line it filtered
//! out, so a marker that is too strict shows up as skipped records rather than as missing
//! usage.
//!
//! Markers are matched as raw bytes, so they must tolerate JSON whitespace: use a key name
//! such as `"usage"` rather than `"usage":`, which a writer may format with a space.

use memchr::memmem::Finder;

/// Whether a line must contain every marker or just one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    All,
    Any,
}

/// A reusable substring prefilter for newline-delimited JSON.
#[derive(Clone, Debug)]
pub struct LinePrefilter {
    finders: Vec<Finder<'static>>,
    mode: Mode,
}

impl LinePrefilter {
    /// A prefilter that admits lines containing every marker.
    pub fn all(markers: &[&[u8]]) -> Self {
        Self::new(markers, Mode::All)
    }

    /// A prefilter that admits lines containing at least one marker.
    pub fn any(markers: &[&[u8]]) -> Self {
        Self::new(markers, Mode::Any)
    }

    fn new(markers: &[&[u8]], mode: Mode) -> Self {
        // `Finder::new` borrows the needle, so own a copy: the prefilter outlives the
        // caller's markers and is built once per scan rather than per line.
        let finders = markers.iter().map(|marker| Finder::new(marker).into_owned()).collect();
        Self { finders, mode }
    }

    /// Whether `line` is worth parsing. An empty marker list admits everything.
    pub fn matches(&self, line: &[u8]) -> bool {
        match self.mode {
            Mode::All => self.finders.iter().all(|finder| finder.find(line).is_some()),
            Mode::Any => {
                self.finders.is_empty()
                    || self.finders.iter().any(|finder| finder.find(line).is_some())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LinePrefilter;

    #[test]
    fn all_requires_every_marker() {
        let prefilter = LinePrefilter::all(&[b"\"usage\"", b"\"message\""]);
        assert!(prefilter.matches(br#"{"message":{"usage":{"input_tokens":1}}}"#));
        assert!(!prefilter.matches(br#"{"message":{"role":"user"}}"#));
        assert!(!prefilter.matches(br#"{"usage":{"input_tokens":1}}"#));
    }

    #[test]
    fn any_requires_one_marker() {
        let prefilter = LinePrefilter::any(&[b"\"token_count\"", b"\"token_usage_record\""]);
        assert!(prefilter.matches(br#"{"type":"token_count"}"#));
        assert!(prefilter.matches(br#"{"type":"token_usage_record"}"#));
        assert!(!prefilter.matches(br#"{"type":"response_item"}"#));
        assert!(LinePrefilter::any(&[]).matches(b"anything"));
    }

    #[test]
    fn markers_without_punctuation_tolerate_writer_whitespace() {
        let strict = LinePrefilter::all(&[b"\"usage\":"]);
        let tolerant = LinePrefilter::all(&[b"\"usage\""]);
        let spaced = br#"{"usage" : {"input_tokens": 1}}"#;
        assert!(!strict.matches(spaced));
        assert!(tolerant.matches(spaced));
    }
}

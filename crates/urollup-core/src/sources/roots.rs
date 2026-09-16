//! Walking declared roots for source files (design §2.1, §2.2).
//!
//! [`discover`] walks each declared root in a deterministic order and returns the logical
//! sources it found, pairing a `.jsonl` file with its `.jsonl.zst` twin under one locator.
//! Its boundaries are the ones the design states:
//!
//! - **Symlinks are followed only within declared roots.** A link whose target resolves
//!   outside every root is reported as [`SkippedLinkReason::OutsideRoots`], a dangling one
//!   as [`SkippedLinkReason::Broken`], and a link back into a directory already walked as
//!   [`SkippedLinkReason::AlreadyVisited`], which also stops a loop.
//! - **Nothing is dropped silently.** A directory that cannot be read becomes an
//!   [`UnreadableEntry`], never an empty directory: `filter_map(Result::ok)` would turn "I
//!   could not read this" into "there is nothing here". A root that does not exist is
//!   listed in [`Discovery::missing_roots`] for the caller to decide about, since a
//!   missing default root is skipped while a missing named one is an error.
//! - **Locators are stable and path-shaped.** A locator is the root-relative path with
//!   `/` separators and any `.zst` suffix removed, so a source keeps its ID when it is
//!   compressed. Bytes that are not UTF-8 are percent-escaped rather than replaced, so two
//!   different names cannot collide into one locator.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::sources::manifest::{Representation, SkippedLink, SkippedLinkReason};
use crate::sources::reader::LogicalSource;

/// One logical source found under a root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredSource {
    /// The root it was found under.
    pub root: PathBuf,
    /// The root-relative locator, without any compression suffix.
    pub locator: String,
    /// Its plain file, compressed file, or both.
    pub files: LogicalSource,
}

/// A path the walk could not read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnreadableEntry {
    /// The path.
    pub path: PathBuf,
    /// Why it could not be read.
    pub kind: io::ErrorKind,
}

/// What one walk of the declared roots found.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Discovery {
    /// Logical sources, ordered by root then locator.
    pub sources: Vec<DiscoveredSource>,
    /// Symlinks that were not followed.
    pub skipped_links: Vec<SkippedLink>,
    /// Roots that do not exist.
    pub missing_roots: Vec<PathBuf>,
    /// Paths that could not be read.
    pub unreadable: Vec<UnreadableEntry>,
    /// Further paths that reach a file already discovered, such as a second link to it.
    pub duplicate_paths: Vec<PathBuf>,
}

/// Walks `roots` and groups the `.jsonl` and `.jsonl.zst` files under them.
pub fn discover(roots: &[PathBuf]) -> Discovery {
    let mut discovery = Discovery::default();
    let mut canonical_roots = Vec::new();
    let mut declared = Vec::new();
    for root in roots {
        match fs::canonicalize(root) {
            Ok(canonical) => {
                canonical_roots.push(canonical);
                declared.push(root.clone());
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                discovery.missing_roots.push(root.clone());
            }
            Err(error) => {
                discovery
                    .unreadable
                    .push(UnreadableEntry { path: root.clone(), kind: error.kind() });
            }
        }
    }

    let mut seen_files: BTreeSet<PathBuf> = BTreeSet::new();
    let mut seen_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for (root, canonical_root) in declared.iter().zip(&canonical_roots) {
        let mut found: BTreeMap<String, LogicalSource> = BTreeMap::new();
        walk(
            root,
            canonical_root,
            &canonical_roots,
            &mut Walk {
                discovery: &mut discovery,
                found: &mut found,
                seen_files: &mut seen_files,
                seen_dirs: &mut seen_dirs,
            },
        );
        for (locator, files) in found {
            discovery.sources.push(DiscoveredSource { root: root.clone(), locator, files });
        }
    }
    discovery
}

struct Walk<'a> {
    discovery: &'a mut Discovery,
    found: &'a mut BTreeMap<String, LogicalSource>,
    seen_files: &'a mut BTreeSet<PathBuf>,
    seen_dirs: &'a mut BTreeSet<PathBuf>,
}

/// Walks one root: every real path first, then the symlinks it holds, so a file reached
/// both directly and through a link is recorded under its real path and the link is a
/// duplicate rather than a second source. Across roots the earlier declared root wins.
fn walk(root: &Path, canonical_root: &Path, canonical_roots: &[PathBuf], state: &mut Walk<'_>) {
    let mut stack = vec![root.to_owned()];
    let mut links: Vec<PathBuf> = Vec::new();
    state.seen_dirs.insert(canonical_root.to_owned());
    loop {
        while let Some(directory) = stack.pop() {
            for path in read_sorted(&directory, state) {
                let file_type = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata.file_type(),
                    Err(error) => {
                        state
                            .discovery
                            .unreadable
                            .push(UnreadableEntry { path, kind: error.kind() });
                        continue;
                    }
                };
                if file_type.is_symlink() {
                    links.push(path);
                } else if file_type.is_dir() {
                    match fs::canonicalize(&path) {
                        Ok(canonical) => {
                            if state.seen_dirs.insert(canonical) {
                                stack.push(path);
                            }
                        }
                        Err(error) => {
                            state
                                .discovery
                                .unreadable
                                .push(UnreadableEntry { path, kind: error.kind() });
                        }
                    }
                } else if file_type.is_file() {
                    match fs::canonicalize(&path) {
                        Ok(canonical) => record_file(&path, &canonical, root, state),
                        Err(error) => {
                            state
                                .discovery
                                .unreadable
                                .push(UnreadableEntry { path, kind: error.kind() });
                        }
                    }
                }
            }
        }
        if links.is_empty() {
            return;
        }
        let mut deferred = std::mem::take(&mut links);
        deferred.sort();
        for path in deferred {
            let Some(target) = resolve_link(&path, canonical_roots, state) else { continue };
            if target.is_dir() {
                if state.seen_dirs.insert(target) {
                    stack.push(path);
                } else {
                    state
                        .discovery
                        .skipped_links
                        .push(SkippedLink { path, reason: SkippedLinkReason::AlreadyVisited });
                }
            } else {
                record_file(&path, &target, root, state);
            }
        }
    }
}

/// One directory's entries, sorted by path; an entry that cannot be read is reported.
fn read_sorted(directory: &Path, state: &mut Walk<'_>) -> Vec<PathBuf> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            state
                .discovery
                .unreadable
                .push(UnreadableEntry { path: directory.to_owned(), kind: error.kind() });
            return Vec::new();
        }
    };
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(error) => state
                .discovery
                .unreadable
                .push(UnreadableEntry { path: directory.to_owned(), kind: error.kind() }),
        }
    }
    paths.sort();
    paths
}

/// Resolves a symlink, reporting it instead of following it when it is broken or leaves
/// the declared roots.
fn resolve_link(path: &Path, canonical_roots: &[PathBuf], state: &mut Walk<'_>) -> Option<PathBuf> {
    match fs::canonicalize(path) {
        Ok(target) => {
            if canonical_roots.iter().any(|root| target.starts_with(root)) {
                Some(target)
            } else {
                state.discovery.skipped_links.push(SkippedLink {
                    path: path.to_owned(),
                    reason: SkippedLinkReason::OutsideRoots,
                });
                None
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            state
                .discovery
                .skipped_links
                .push(SkippedLink { path: path.to_owned(), reason: SkippedLinkReason::Broken });
            None
        }
        Err(error) => {
            state
                .discovery
                .unreadable
                .push(UnreadableEntry { path: path.to_owned(), kind: error.kind() });
            None
        }
    }
}

fn record_file(path: &Path, canonical: &Path, root: &Path, state: &mut Walk<'_>) {
    let Some(representation) = representation_of(path) else { return };
    if !state.seen_files.insert(canonical.to_owned()) {
        state.discovery.duplicate_paths.push(path.to_owned());
        return;
    }
    let Some(relative) = path.strip_prefix(root).ok() else { return };
    let locator = locator_for(relative);
    let files =
        state.found.entry(locator).or_insert(LogicalSource { plain: None, compressed: None });
    match representation {
        Representation::Plain => files.plain = Some(path.to_owned()),
        Representation::Zstd => files.compressed = Some(path.to_owned()),
    }
}

/// `.jsonl` and `.jsonl.zst` are source files; anything else is not.
fn representation_of(path: &Path) -> Option<Representation> {
    let name = path.file_name()?.to_string_lossy();
    if name.ends_with(".jsonl") {
        Some(Representation::Plain)
    } else if name.ends_with(".jsonl.zst") {
        Some(Representation::Zstd)
    } else {
        None
    }
}

/// The root-relative locator: `/` separators, no `.zst` suffix, and percent-escaped bytes
/// that are not UTF-8, so two names cannot collide into one locator.
pub fn locator_for(relative: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(escape_component(part)),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                parts.push(escape_component(component.as_os_str()));
            }
        }
    }
    let joined = parts.join("/");
    let trimmed = joined.strip_suffix(".zst").map(str::to_owned);
    trimmed.unwrap_or(joined)
}

fn escape_component(part: &OsStr) -> String {
    match part.to_str() {
        Some(text) => text.replace('%', "%25"),
        None => escape_bytes(part),
    }
}

#[cfg(unix)]
fn escape_bytes(part: &OsStr) -> String {
    use std::fmt::Write as _;
    use std::os::unix::ffi::OsStrExt as _;
    let mut out = String::new();
    let mut bytes = part.as_bytes();
    loop {
        match std::str::from_utf8(bytes) {
            Ok(text) => {
                out.push_str(&text.replace('%', "%25"));
                return out;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                if let Some(text) =
                    bytes.get(..valid).and_then(|head| std::str::from_utf8(head).ok())
                {
                    out.push_str(&text.replace('%', "%25"));
                }
                let skip = error.error_len().unwrap_or(1).max(1);
                for byte in bytes.iter().skip(valid).take(skip) {
                    // Writing to a String cannot fail.
                    let _ = write!(out, "%{byte:02X}");
                }
                match bytes.get(valid.saturating_add(skip)..) {
                    Some(rest) => bytes = rest,
                    None => return out,
                }
            }
        }
    }
}

// On Windows a name that is not valid UTF-8 holds unpaired surrogates, which have no byte
// form to escape; the lossy text is used, and two such names could share a locator.
#[cfg(not(unix))]
fn escape_bytes(part: &OsStr) -> String {
    part.to_string_lossy().replace('%', "%25")
}

#[cfg(test)]
mod tests;

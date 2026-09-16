use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{Discovery, discover, locator_for};
use crate::sources::manifest::SkippedLinkReason;

fn write(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, b"{\"i\":1}\n").unwrap();
}

fn locators(discovery: &Discovery) -> Vec<&str> {
    discovery.sources.iter().map(|source| source.locator.as_str()).collect()
}

#[test]
fn finds_source_files_under_a_root_in_a_stable_order() {
    let root = TempDir::new().unwrap();
    for name in ["b/second.jsonl", "a/first.jsonl", "a/notes.md", "a/deeper/third.jsonl"] {
        write(&root.path().join(name));
    }
    let discovery = discover(&[root.path().to_owned()]);
    assert_eq!(
        locators(&discovery),
        vec!["a/deeper/third.jsonl", "a/first.jsonl", "b/second.jsonl"]
    );
    assert!(discovery.skipped_links.is_empty());
    assert!(discovery.unreadable.is_empty());
    assert_eq!(discover(&[root.path().to_owned()]), discovery, "a second walk finds the same tree");
}

#[test]
fn a_plain_file_and_its_compressed_twin_share_one_locator() {
    let root = TempDir::new().unwrap();
    write(&root.path().join("s.jsonl"));
    write(&root.path().join("s.jsonl.zst"));
    write(&root.path().join("archived/old.jsonl.zst"));

    let discovery = discover(&[root.path().to_owned()]);
    assert_eq!(locators(&discovery), vec!["archived/old.jsonl", "s.jsonl"]);
    let twinned = &discovery.sources[1].files;
    assert_eq!(twinned.plain, Some(root.path().join("s.jsonl")));
    assert_eq!(twinned.compressed, Some(root.path().join("s.jsonl.zst")));
    let compressed_only = &discovery.sources[0].files;
    assert_eq!(compressed_only.plain, None);
    assert!(compressed_only.compressed.is_some());
}

#[test]
fn a_missing_root_is_reported_rather_than_failing_the_walk() {
    let root = TempDir::new().unwrap();
    write(&root.path().join("s.jsonl"));
    let missing = root.path().join("nowhere");
    let discovery = discover(&[missing.clone(), root.path().to_owned()]);
    assert_eq!(discovery.missing_roots, vec![missing]);
    assert_eq!(locators(&discovery), vec!["s.jsonl"]);
}

#[cfg(unix)]
mod links {
    use std::os::unix::fs::symlink;

    use super::{Path, PathBuf, SkippedLinkReason, TempDir, discover, fs, locators, write};

    fn link(original: &Path, link: &PathBuf) {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        symlink(original, link).unwrap();
    }

    #[test]
    fn links_inside_the_roots_are_followed_and_links_that_leave_are_reported() {
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        write(&root.path().join("real/inside.jsonl"));
        write(&outside.path().join("elsewhere.jsonl"));
        link(&root.path().join("real/inside.jsonl"), &root.path().join("linked.jsonl"));
        link(&outside.path().join("elsewhere.jsonl"), &root.path().join("outside.jsonl"));
        link(&root.path().join("real"), &root.path().join("linked-dir"));
        link(&root.path().join("gone.jsonl"), &root.path().join("broken.jsonl"));

        let discovery = discover(&[root.path().to_owned()]);
        assert_eq!(locators(&discovery), vec!["real/inside.jsonl"]);
        let mut skipped: Vec<(String, SkippedLinkReason)> = discovery
            .skipped_links
            .iter()
            .map(|skip| {
                (skip.path.file_name().unwrap().to_string_lossy().into_owned(), skip.reason)
            })
            .collect();
        skipped.sort();
        assert_eq!(
            skipped,
            vec![
                ("broken.jsonl".to_owned(), SkippedLinkReason::Broken),
                // The directory link points at a directory this walk already covered.
                ("linked-dir".to_owned(), SkippedLinkReason::AlreadyVisited),
                ("outside.jsonl".to_owned(), SkippedLinkReason::OutsideRoots),
            ]
        );
        // The link to a file already found and the link to a directory already walked are
        // the same source, not a second one.
        assert_eq!(discovery.sources.len(), 1);
        assert!(!discovery.duplicate_paths.is_empty());
    }

    #[test]
    fn a_link_into_another_declared_root_is_followed() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        write(&second.path().join("shared.jsonl"));
        link(&second.path().join("shared.jsonl"), &first.path().join("shared.jsonl"));

        let discovery = discover(&[first.path().to_owned(), second.path().to_owned()]);
        assert!(discovery.skipped_links.is_empty(), "{:?}", discovery.skipped_links);
        assert_eq!(locators(&discovery), vec!["shared.jsonl"]);
        assert_eq!(discovery.duplicate_paths.len(), 1, "the same file is not two sources");
    }

    #[test]
    fn a_link_loop_ends_the_walk() {
        let root = TempDir::new().unwrap();
        write(&root.path().join("nested/s.jsonl"));
        link(root.path(), &root.path().join("nested/loop"));

        let discovery = discover(&[root.path().to_owned()]);
        assert_eq!(locators(&discovery), vec!["nested/s.jsonl"]);
        assert!(
            discovery
                .skipped_links
                .iter()
                .any(|skip| skip.reason == SkippedLinkReason::AlreadyVisited)
        );
    }

    #[test]
    fn an_unreadable_directory_is_reported_not_treated_as_empty() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = TempDir::new().unwrap();
        let closed = root.path().join("closed");
        write(&closed.join("hidden.jsonl"));
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o000)).unwrap();

        let discovery = discover(&[root.path().to_owned()]);
        let reported = discovery.unreadable.iter().any(|entry| entry.path == closed);

        fs::set_permissions(&closed, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(reported, "unreadable {:?}", discovery.unreadable);
        assert!(discovery.sources.is_empty());
    }
}

#[test]
fn locators_use_forward_slashes_and_escape_percent_signs() {
    assert_eq!(locator_for(Path::new("a/b/c.jsonl")), "a/b/c.jsonl");
    assert_eq!(locator_for(Path::new("a/c.jsonl.zst")), "a/c.jsonl");
    assert_eq!(locator_for(Path::new("100%/c.jsonl")), "100%25/c.jsonl");
}

// APFS and NTFS reject a name that is not valid UTF-8, so this checks the locator rule
// itself rather than a file on disk.
#[cfg(unix)]
#[test]
fn a_name_that_is_not_utf_8_becomes_an_escaped_locator() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let escaped = |bytes: &[u8]| locator_for(Path::new(OsStr::from_bytes(bytes)));
    assert_eq!(escaped(b"a\xffb.jsonl"), "a%FFb.jsonl");
    assert_ne!(escaped(b"a\xffb.jsonl"), escaped(b"a\xfeb.jsonl"), "no two names collide");
    assert_eq!(escaped(b"caf\xc3\xa9.jsonl"), "caf\u{e9}.jsonl", "valid UTF-8 is kept");
}

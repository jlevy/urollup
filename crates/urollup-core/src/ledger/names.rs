//! Interned names: model and reasoning-effort strings that ledger rows share.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, PoisonError};

/// A model or reasoning-effort name, interned so a row holds an 8-byte reference instead
/// of an owned string.
///
/// Each distinct text is stored once per process and never freed. The vocabulary is small
/// (a whole history names a few dozen models and efforts), so the table stays a few
/// kilobytes, and a row needs no ledger table to compare or print its names. Equality,
/// ordering and hashing are those of the text, so a row that embeds a `Name` compares as
/// it did with a `String`.
#[derive(Clone, Copy)]
pub struct Name(&'static &'static str);

const _: () = assert!(std::mem::size_of::<Option<Name>>() == 8);

/// Every interned text.
static TABLE: Mutex<BTreeMap<&'static str, Name>> = Mutex::new(BTreeMap::new());

/// Recently interned names per thread, so rows that repeat a name skip the shared lock.
const RECENT_CAPACITY: usize = 8;

thread_local! {
    static RECENT: RefCell<Vec<Name>> = const { RefCell::new(Vec::new()) };
}

impl Name {
    /// The interned name with this text.
    pub fn new(text: &str) -> Self {
        RECENT.with(|recent| {
            let mut recent = recent.borrow_mut();
            if let Some(name) = recent.iter().find(|name| name.as_str() == text) {
                return *name;
            }
            let name = intern(text);
            if recent.len() == RECENT_CAPACITY {
                recent.remove(0);
            }
            recent.push(name);
            name
        })
    }

    /// The name's text.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

fn intern(text: &str) -> Name {
    let mut table = TABLE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(name) = table.get(text) {
        return *name;
    }
    let text: &'static str = Box::leak(Box::<str>::from(text));
    let name = Name(Box::leak(Box::new(text)));
    table.insert(text, name);
    name
}

impl From<&str> for Name {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Name {
    fn from(text: String) -> Self {
        Self::new(&text)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        // One reference per distinct text, so equal texts share their reference.
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Name {}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other { Ordering::Equal } else { self.as_str().cmp(other.as_str()) }
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

    use proptest::prelude::*;

    use super::{Name, RECENT_CAPACITY};

    proptest! {
        #[test]
        fn names_compare_and_hash_like_their_text(
            left in "[a-c]{0,3}",
            right in "[a-c]{0,3}",
        ) {
            let (left_name, right_name) = (Name::from(left.as_str()), Name::from(right.clone()));
            prop_assert_eq!(left_name.as_str(), left.as_str());
            prop_assert_eq!(left_name == right_name, left == right);
            prop_assert_eq!(left_name.cmp(&right_name), left.cmp(&right));
            let hasher = BuildHasherDefault::<DefaultHasher>::default();
            prop_assert_eq!(hasher.hash_one(left_name), hasher.hash_one(&left));
            prop_assert_eq!(format!("{left_name:?}"), format!("{left:?}"));
        }
    }

    #[test]
    fn a_name_is_interned_once_across_threads_and_evictions() {
        let first = Name::new("model-under-test");
        for index in 0..=RECENT_CAPACITY {
            Name::new(&format!("filler-{index}"));
        }
        let from_other_thread =
            std::thread::spawn(|| Name::new("model-under-test")).join().expect("thread runs");
        assert!(std::ptr::eq(first.0, Name::new("model-under-test").0));
        assert!(std::ptr::eq(first.0, from_other_thread.0));
        assert_eq!(first, "model-under-test");
    }
}

//! Linux memory allowances: cgroup v2 and v1 limits and the soft data and address-space
//! rlimits, read from `/proc` and the cgroup filesystems below a configurable root.
//!
//! Every file is optional. A missing, unreadable or malformed file contributes nothing,
//! so a host without cgroups or with an unusual layout keeps its physical RAM.

use std::fs;
use std::path::{Path, PathBuf};

use super::MemorySource;

/// A cgroup v1 `hierarchical_memory_limit` at or above this is the kernel's "no limit"
/// sentinel: the page counter maximum, `LONG_MAX` rounded down to a page, which is
/// `0x7FFF_FFFF_FFFF_F000` on 4 KiB pages and only smaller by one page on larger ones.
const V1_UNLIMITED_FLOOR: u64 = 1 << 62;

/// Every allowance discoverable below `root`, which stands in for `/`.
///
/// `physical_ram` lets a cgroup v1 limit at or above physical RAM count as no limit, which
/// is how v1 reports an unlimited group that sits below the root.
pub(super) fn allowances(root: &Path, physical_ram: Option<u64>) -> Vec<(u64, MemorySource)> {
    let mut found = Vec::new();
    let memberships = read(root, "proc/self/cgroup").map(|text| Memberships::parse(&text));
    let mounts = read(root, "proc/self/mountinfo").map(|text| parse_mounts(&text));
    if let (Some(memberships), Some(mounts)) = (memberships, mounts) {
        if let Some(path) = memberships.unified.as_deref() {
            found.extend(unified_limits(root, &mounts, path));
        }
        if let Some(path) = memberships.memory.as_deref() {
            found.extend(
                v1_limit(root, &mounts, path, physical_ram)
                    .map(|bytes| (bytes, MemorySource::CgroupV1Limit)),
            );
        }
    }
    if let Some(text) = read(root, "proc/self/limits") {
        for (name, source) in [
            ("Max address space", MemorySource::AddressSpaceLimit),
            ("Max data size", MemorySource::DataSizeLimit),
        ] {
            found.extend(soft_limit(&text, name).map(|bytes| (bytes, source)));
        }
    }
    found
}

fn read(root: &Path, relative: &str) -> Option<String> {
    fs::read_to_string(root.join(relative)).ok()
}

/// The process's cgroup paths from `/proc/self/cgroup`.
#[derive(Debug, Default, Eq, PartialEq)]
struct Memberships {
    /// The cgroup v2 path, from the `0::<path>` line.
    unified: Option<String>,
    /// The cgroup v1 path of the hierarchy with the `memory` controller.
    memory: Option<String>,
}

impl Memberships {
    fn parse(text: &str) -> Self {
        let mut memberships = Self::default();
        for line in text.lines() {
            let mut fields = line.splitn(3, ':');
            let (Some(hierarchy), Some(controllers), Some(path)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if !path.starts_with('/') {
                continue;
            }
            if hierarchy == "0" && controllers.is_empty() {
                memberships.unified.get_or_insert_with(|| path.to_owned());
            } else if controllers.split(',').any(|controller| controller == "memory") {
                memberships.memory.get_or_insert_with(|| path.to_owned());
            }
        }
        memberships
    }
}

/// One cgroup filesystem mount from `/proc/self/mountinfo`.
#[derive(Debug, Eq, PartialEq)]
struct CgroupMount {
    /// The cgroup path that the mount point shows.
    root: String,
    /// Where it is mounted, as an absolute path.
    point: String,
    /// `cgroup2`, or a v1 hierarchy that includes the `memory` controller.
    kind: MountKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MountKind {
    Unified,
    MemoryV1,
}

/// Cgroup mounts, in file order. A line is `id parent major:minor root point options
/// [optional fields] - fstype source super-options`.
fn parse_mounts(text: &str) -> Vec<CgroupMount> {
    let mut mounts = Vec::new();
    for line in text.lines() {
        let Some((left, right)) = line.split_once(" - ") else { continue };
        let left: Vec<&str> = left.split(' ').collect();
        let right: Vec<&str> = right.split(' ').collect();
        let (Some(root), Some(point), Some(fstype)) = (left.get(3), left.get(4), right.first())
        else {
            continue;
        };
        let kind = match *fstype {
            "cgroup2" => MountKind::Unified,
            "cgroup"
                if right
                    .get(2)
                    .is_some_and(|options| options.split(',').any(|option| option == "memory")) =>
            {
                MountKind::MemoryV1
            }
            _ => continue,
        };
        let (Some(root), Some(point)) = (unescape(root), unescape(point)) else { continue };
        if root.starts_with('/') && point.starts_with('/') {
            mounts.push(CgroupMount { root, point, kind });
        }
    }
    mounts
}

/// Decodes the octal escapes mountinfo writes for space, tab, newline and backslash.
fn unescape(field: &str) -> Option<String> {
    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'\\' {
            let digits = bytes.get(index + 1..index + 4)?;
            if !digits.iter().all(|digit| (b'0'..=b'7').contains(digit)) {
                return None;
            }
            let value =
                digits.iter().fold(0_u32, |value, digit| value * 8 + u32::from(digit - b'0'));
            decoded.push(u8::try_from(value).ok()?);
            index += 4;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// The directory below `root` that holds cgroup `path` on `mount`, if the mount shows it.
fn cgroup_directory(root: &Path, mount: &CgroupMount, path: &str) -> Option<(PathBuf, PathBuf)> {
    let relative = if mount.root == "/" {
        path.trim_start_matches('/')
    } else if path == mount.root {
        ""
    } else {
        path.strip_prefix(mount.root.as_str())?.strip_prefix('/')?
    };
    let point = root.join(mount.point.trim_start_matches('/'));
    let directory = relative
        .split('/')
        .filter(|part| !part.is_empty())
        .try_fold(point.clone(), |directory, part| {
            (part != "." && part != "..").then(|| directory.join(part))
        })?;
    Some((point, directory))
}

/// `memory.max` and `memory.high` of the process's cgroup and every ancestor up to the
/// mount point.
fn unified_limits(root: &Path, mounts: &[CgroupMount], path: &str) -> Vec<(u64, MemorySource)> {
    let Some((point, directory)) = mounts
        .iter()
        .filter(|mount| mount.kind == MountKind::Unified)
        .find_map(|mount| cgroup_directory(root, mount, path))
    else {
        return Vec::new();
    };
    let mut limits = Vec::new();
    let mut current = Some(directory.as_path());
    while let Some(directory) = current {
        for (file, source) in
            [("memory.max", MemorySource::CgroupMax), ("memory.high", MemorySource::CgroupHigh)]
        {
            let value = fs::read_to_string(directory.join(file)).ok();
            limits.extend(value.as_deref().and_then(unified_value).map(|bytes| (bytes, source)));
        }
        current = (directory != point.as_path()).then(|| directory.parent()).flatten();
    }
    limits
}

/// A `memory.max` or `memory.high` value: a byte count, or `max` for none.
fn unified_value(text: &str) -> Option<u64> {
    let text = text.trim();
    if text == "max" {
        return None;
    }
    whole_number(text)
}

/// The v1 `hierarchical_memory_limit` of the process's memory cgroup, which already
/// includes every ancestor's limit; the unlimited sentinel and values at or above
/// physical RAM are no limit.
fn v1_limit(
    root: &Path,
    mounts: &[CgroupMount],
    path: &str,
    physical_ram: Option<u64>,
) -> Option<u64> {
    let (_, directory) = mounts
        .iter()
        .filter(|mount| mount.kind == MountKind::MemoryV1)
        .find_map(|mount| cgroup_directory(root, mount, path))?;
    let stat = fs::read_to_string(directory.join("memory.stat")).ok()?;
    let value = stat.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        (fields.next() == Some("hierarchical_memory_limit")).then(|| fields.next()).flatten()
    })?;
    let bytes = whole_number(value)?;
    let unlimited = bytes >= V1_UNLIMITED_FLOOR || physical_ram.is_some_and(|ram| bytes >= ram);
    (!unlimited).then_some(bytes)
}

/// The soft limit of the `/proc/self/limits` row named `name`, or `None` when it is
/// `unlimited`, absent or malformed.
fn soft_limit(text: &str, name: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(name)?;
        if !rest.starts_with(' ') {
            return None;
        }
        whole_number(rest.split_whitespace().next()?)
    })
}

fn whole_number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memberships_read_the_unified_and_memory_lines() {
        let text = "12:cpu,cpuacct:/a\n4:memory:/docker/abc\n0::/user.slice/session.scope\n";
        assert_eq!(
            Memberships::parse(text),
            Memberships {
                unified: Some("/user.slice/session.scope".into()),
                memory: Some("/docker/abc".into()),
            }
        );
        assert_eq!(Memberships::parse("garbage\n0::relative\n"), Memberships::default());
    }

    #[test]
    fn mountinfo_keeps_cgroup2_and_memory_v1_mounts() {
        let text = concat!(
            "22 1 0:21 / /proc rw - proc proc rw\n",
            "30 25 0:26 / /sys/fs/cgroup rw,nosuid shared:4 - cgroup2 cgroup2 rw\n",
            "31 25 0:27 /docker/abc /sys/fs/cgroup/memory rw - cgroup cgroup rw,memory\n",
            "32 25 0:28 / /sys/fs/cgroup/cpu rw - cgroup cgroup rw,cpu,cpuacct\n",
            "33 25 0:29 / /mnt/with\\040space rw - cgroup2 cgroup2 rw\n",
            "malformed line without separator\n",
        );
        let mounts = parse_mounts(text);
        assert_eq!(
            mounts,
            vec![
                CgroupMount {
                    root: "/".into(),
                    point: "/sys/fs/cgroup".into(),
                    kind: MountKind::Unified
                },
                CgroupMount {
                    root: "/docker/abc".into(),
                    point: "/sys/fs/cgroup/memory".into(),
                    kind: MountKind::MemoryV1
                },
                CgroupMount {
                    root: "/".into(),
                    point: "/mnt/with space".into(),
                    kind: MountKind::Unified
                },
            ]
        );
    }

    #[test]
    fn a_cgroup_outside_its_mount_root_is_not_located() {
        let mount = CgroupMount {
            root: "/docker/abc".into(),
            point: "/sys".into(),
            kind: MountKind::Unified,
        };
        let root = Path::new("/r");
        assert_eq!(cgroup_directory(root, &mount, "/other"), None);
        assert_eq!(cgroup_directory(root, &mount, "/docker/abcdef"), None);
        assert_eq!(
            cgroup_directory(root, &mount, "/docker/abc/child"),
            Some((PathBuf::from("/r/sys"), PathBuf::from("/r/sys/child")))
        );
        let at_root = CgroupMount { root: "/".into(), ..mount };
        assert_eq!(cgroup_directory(root, &at_root, "/a/../b"), None);
    }

    #[test]
    fn soft_limits_ignore_unlimited_and_malformed_rows() {
        let text = concat!(
            "Limit                     Soft Limit           Hard Limit           Units     \n",
            "Max data size             unlimited            unlimited            bytes     \n",
            "Max address space         8589934592           unlimited            bytes     \n",
        );
        assert_eq!(soft_limit(text, "Max address space"), Some(8 << 30));
        assert_eq!(soft_limit(text, "Max data size"), None);
        assert_eq!(
            soft_limit("Max address space 12x unlimited bytes\n", "Max address space"),
            None
        );
        assert_eq!(soft_limit("Max address spacey 12 12 bytes\n", "Max address space"), None);
    }
}

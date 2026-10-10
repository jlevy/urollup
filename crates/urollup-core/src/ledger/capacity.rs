//! Per-agent observation-row ceiling derived from a RAM budget, and the effective memory
//! and whole-process budget that replace it (scalable-ingestion plan, "Process-Wide
//! Admission").
//!
//! Decode admission bounds retained request-bearing rows; reconciliation checks again
//! before request construction. This is a row-shell budget, not a process memory cap.
//! The default is 25% of
//! physical RAM, falling back to 2 GiB when RAM cannot be read. `--max-ram` and
//! `UROLLUP_MAX_RAM` parse a byte size or a percent; `--max-rows` is an exact count.
//!
//! [`effective_memory`] is the smallest of physical RAM and, on Linux, the cgroup v2
//! `memory.max` and `memory.high` of the process's cgroup and its ancestors, the cgroup v1
//! `hierarchical_memory_limit`, and the soft address-space and data-size rlimits.
//! [`MemoryBudget`] takes 25% of it by default and names where the number came from.
//! The CLI still uses the row ceiling until process-wide admission is wired in.

mod linux;

use std::fmt;
use std::mem::size_of;
use std::path::Path;
use std::sync::OnceLock;

use super::reconcile::RequestObservation;

/// Row-shell budget used when the default physical RAM query fails.
pub const FALLBACK_BUDGET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Default share of physical RAM used as the ingest budget.
pub const DEFAULT_RAM_PERCENT: u8 = 25;

/// The most request observations that fit in [`FALLBACK_BUDGET_BYTES`].
pub const FALLBACK_MAX_OBSERVATIONS: usize =
    2 * 1024 * 1024 * 1024 / size_of::<RequestObservation>();

/// A per-agent observation ceiling and the label the capacity diagnostic names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationCapacity {
    maximum: usize,
    label: String,
}

impl ObservationCapacity {
    /// 25% of `physical_ram`, or 2 GiB when RAM is unknown.
    pub fn default_for_ram(physical_ram: Option<u64>) -> Self {
        match physical_ram {
            Some(ram) => {
                let bytes = ram.saturating_mul(u64::from(DEFAULT_RAM_PERCENT)) / 100;
                Self::from_byte_budget(bytes, format!("25% of RAM ({})", format_byte_budget(bytes)))
            }
            None => Self::fallback(),
        }
    }

    /// The 2 GiB fallback used when physical RAM cannot be read.
    pub fn fallback() -> Self {
        Self::from_byte_budget(FALLBACK_BUDGET_BYTES, "2 GiB")
    }

    /// Whole observation rows that fit in `bytes`.
    pub fn from_byte_budget(bytes: u64, label: impl Into<String>) -> Self {
        Self { maximum: rows_for_budget(bytes), label: label.into() }
    }

    /// An exact row-count ceiling.
    pub fn from_rows(maximum: usize) -> Self {
        Self { maximum, label: format!("{maximum} rows") }
    }

    /// The most observations one reconciliation accepts.
    pub fn maximum(&self) -> usize {
        self.maximum
    }

    /// The budget name shown in [`super::reconcile::ReconcileError::CapacityExceeded`].
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Default for ObservationCapacity {
    fn default() -> Self {
        Self::default_for_ram(physical_memory_bytes())
    }
}

/// A parsed `--max-ram` / `UROLLUP_MAX_RAM` value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RamBudget {
    /// A percent of physical RAM, from 1 to 100.
    Percent(u8),
    /// An exact byte budget.
    Bytes(u64),
}

impl RamBudget {
    /// Parses a byte size (`512M`, `8G`, `8GiB`) or a percent (`25%`).
    pub fn parse(text: &str) -> Result<Self, RamBudgetError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(RamBudgetError::Empty);
        }
        if let Some(percent) = text.strip_suffix('%') {
            let percent = percent.trim();
            let value = parse_count(percent, text)?;
            let percent = u8::try_from(value).map_err(|_| RamBudgetError::PercentOutOfRange {
                value,
                source: text.to_owned(),
            })?;
            if !(1..=100).contains(&percent) {
                return Err(RamBudgetError::PercentOutOfRange { value, source: text.to_owned() });
            }
            return Ok(Self::Percent(percent));
        }
        let split = text.find(|byte: char| !byte.is_ascii_digit()).unwrap_or(text.len());
        let (number, unit) = text.split_at(split);
        let count = parse_count(number, text)?;
        let multiplier = unit_multiplier(unit.trim(), text)?;
        let bytes = count
            .checked_mul(multiplier)
            .ok_or_else(|| RamBudgetError::Overflow { source: text.to_owned() })?;
        if bytes == 0 {
            return Err(RamBudgetError::Zero { source: text.to_owned() });
        }
        Ok(Self::Bytes(bytes))
    }

    /// Converts this budget into a row ceiling for [`RequestObservation`].
    pub fn to_capacity(
        self,
        physical_ram: Option<u64>,
    ) -> Result<ObservationCapacity, RamBudgetError> {
        match self {
            Self::Percent(percent) => {
                let Some(ram) = physical_ram else {
                    return Err(RamBudgetError::UnknownPhysicalMemory { percent });
                };
                let bytes = ram.saturating_mul(u64::from(percent)) / 100;
                if bytes == 0 {
                    return Err(RamBudgetError::Zero {
                        source: format!("{percent}% of {ram} bytes"),
                    });
                }
                Ok(ObservationCapacity::from_byte_budget(
                    bytes,
                    format!("{percent}% of RAM ({})", format_byte_budget(bytes)),
                ))
            }
            Self::Bytes(bytes) => {
                Ok(ObservationCapacity::from_byte_budget(bytes, format_byte_budget(bytes)))
            }
        }
    }
}

/// Why a RAM budget could not be parsed or applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RamBudgetError {
    /// The value was empty after trimming.
    Empty,
    /// The text was not a whole number plus a known unit or `%`.
    Invalid {
        /// The rejected text.
        source: String,
    },
    /// A percent was outside 1–100.
    PercentOutOfRange {
        /// The parsed integer.
        value: u64,
        /// The rejected text.
        source: String,
    },
    /// The size overflowed `u64`.
    Overflow {
        /// The rejected text.
        source: String,
    },
    /// The budget was zero bytes.
    Zero {
        /// The rejected text.
        source: String,
    },
    /// A percent was given but physical RAM could not be read.
    UnknownPhysicalMemory {
        /// The requested percent.
        percent: u8,
    },
    /// A percent of the whole-process budget was given, but no effective memory size
    /// could be discovered.
    UnknownEffectiveMemory {
        /// The requested percent.
        percent: u8,
    },
}

impl fmt::Display for RamBudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "RAM budget must not be empty"),
            Self::Invalid { source } => write!(
                f,
                "RAM budget {source:?} must be a byte size such as 512M, 8G or 8GiB, or a percent such as 25%"
            ),
            Self::PercentOutOfRange { source, .. } => {
                write!(f, "RAM budget {source:?} must be a percent from 1% to 100%")
            }
            Self::Overflow { source } => {
                write!(f, "RAM budget {source:?} is larger than 2^64 bytes")
            }
            Self::Zero { source } => write!(f, "RAM budget {source:?} must be greater than zero"),
            Self::UnknownPhysicalMemory { percent } => write!(
                f,
                "cannot apply {percent}% because physical RAM is unknown; pass a byte size such as 2G or --max-rows"
            ),
            Self::UnknownEffectiveMemory { percent } => write!(
                f,
                "cannot apply {percent}% because this machine's memory size is unknown; pass a byte size such as 2G"
            ),
        }
    }
}

impl std::error::Error for RamBudgetError {}

/// Resolves the per-agent ceiling from optional `--max-ram` / env and `--max-rows`.
///
/// `--max-rows` alone is an exact override of the default. When both a RAM budget and a
/// row count are set, the stricter (smaller) ceiling wins.
pub fn resolve_capacity(
    ram: Option<RamBudget>,
    max_rows: Option<u64>,
    physical_ram: Option<u64>,
) -> Result<ObservationCapacity, RamBudgetError> {
    let from_rows = max_rows.map(|rows| {
        let maximum = usize::try_from(rows).unwrap_or(usize::MAX);
        ObservationCapacity::from_rows(maximum)
    });
    match (ram, from_rows) {
        (None, None) => Ok(ObservationCapacity::default_for_ram(physical_ram)),
        (Some(spec), None) => spec.to_capacity(physical_ram),
        (None, Some(rows)) => Ok(rows),
        (Some(spec), Some(rows)) => {
            let from_ram = spec.to_capacity(physical_ram)?;
            Ok(if rows.maximum() <= from_ram.maximum() { rows } else { from_ram })
        }
    }
}

/// Whole [`RequestObservation`] rows that fit in `bytes`.
pub fn rows_for_budget(bytes: u64) -> usize {
    let row = u64::try_from(size_of::<RequestObservation>()).unwrap_or(u64::MAX);
    if row == 0 {
        return 0;
    }
    usize::try_from(bytes / row).unwrap_or(usize::MAX)
}

/// What bounds the memory this process may use.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemorySource {
    /// Physical RAM: `hw.memsize`, `/proc/meminfo` `MemTotal` or `GlobalMemoryStatusEx`.
    PhysicalRam,
    /// A cgroup v2 `memory.max` of the process's cgroup or an ancestor.
    CgroupMax,
    /// A cgroup v2 `memory.high` of the process's cgroup or an ancestor.
    CgroupHigh,
    /// The cgroup v1 `hierarchical_memory_limit` of the process's memory cgroup.
    CgroupV1Limit,
    /// The soft `RLIMIT_AS` (`Max address space`).
    AddressSpaceLimit,
    /// The soft `RLIMIT_DATA` (`Max data size`).
    DataSizeLimit,
}

/// The smallest discoverable memory allowance and what set it.
///
/// Available or free memory never counts: it changes between runs, and admission must
/// decide the same way for the same input and budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveMemory {
    /// The allowance in bytes.
    pub bytes: u64,
    /// What set it.
    pub source: MemorySource,
}

impl EffectiveMemory {
    /// The smallest of `physical_ram` and `allowances`. Physical RAM wins a tie, and
    /// otherwise the first smallest allowance does, so the label is deterministic.
    pub fn smallest(
        physical_ram: Option<u64>,
        allowances: impl IntoIterator<Item = (u64, MemorySource)>,
    ) -> Option<Self> {
        physical_ram
            .map(|bytes| (bytes, MemorySource::PhysicalRam))
            .into_iter()
            .chain(allowances)
            .fold(None, |smallest: Option<Self>, (bytes, source)| match smallest {
                Some(current) if current.bytes <= bytes => Some(current),
                _ => Some(Self { bytes, source }),
            })
    }

    /// The allowance as a label names it, such as `32 GiB physical RAM` or
    /// `the 4 GiB cgroup limit`, rounded down.
    pub fn describe(&self) -> String {
        let size = format_memory(self.bytes, Rounding::Down);
        match self.source {
            MemorySource::PhysicalRam => format!("{size} physical RAM"),
            MemorySource::CgroupMax | MemorySource::CgroupV1Limit => {
                format!("the {size} cgroup limit")
            }
            MemorySource::CgroupHigh => format!("the {size} cgroup memory.high limit"),
            MemorySource::AddressSpaceLimit => format!("the {size} address-space limit"),
            MemorySource::DataSizeLimit => format!("the {size} data-size limit"),
        }
    }
}

/// This process's effective memory, discovered once: physical RAM and, on Linux, the
/// cgroup and rlimit allowances. `None` when nothing can be read.
///
/// macOS and Windows use physical RAM; Windows job-object limits are not read.
pub fn effective_memory() -> Option<EffectiveMemory> {
    static CACHED: OnceLock<Option<EffectiveMemory>> = OnceLock::new();
    *CACHED.get_or_init(|| {
        #[cfg(target_os = "linux")]
        let memory = effective_memory_under(Path::new("/"), physical_memory_bytes());
        #[cfg(not(target_os = "linux"))]
        let memory = EffectiveMemory::smallest(physical_memory_bytes(), []);
        memory
    })
}

/// The Linux discovery of [`effective_memory`], reading `proc/self/cgroup`,
/// `proc/self/mountinfo`, `proc/self/limits` and the cgroup files below `root` instead of
/// `/`, with `physical_ram` supplied by the caller.
///
/// Missing, unreadable and malformed files are ignored. A cgroup v2 value of `max`, a v1
/// value at or above physical RAM (or the kernel's unlimited sentinel), and an
/// `unlimited` rlimit are no allowance.
pub fn effective_memory_under(root: &Path, physical_ram: Option<u64>) -> Option<EffectiveMemory> {
    EffectiveMemory::smallest(physical_ram, linux::allowances(root, physical_ram))
}

/// The whole-process memory budget `B`, the label that names its source, and the
/// effective memory it was taken from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryBudget {
    bytes: u64,
    label: String,
    memory: Option<EffectiveMemory>,
}

impl MemoryBudget {
    /// 25% of `memory`, or [`FALLBACK_BUDGET_BYTES`] when it is unknown.
    pub fn default_for(memory: Option<EffectiveMemory>) -> Self {
        match memory {
            Some(memory) => Self::percent_of(DEFAULT_RAM_PERCENT, memory),
            None => Self::exact(
                FALLBACK_BUDGET_BYTES,
                format!(
                    "{} fallback (physical RAM unknown)",
                    format_memory(FALLBACK_BUDGET_BYTES, Rounding::Down)
                ),
            ),
        }
    }

    /// An exact budget, such as an explicit `--max-ram` size, labeled `label`.
    pub fn exact(bytes: u64, label: impl Into<String>) -> Self {
        Self { bytes, label: label.into(), memory: None }
    }

    /// `percent` of `memory`, labeled like `25% of 32 GiB physical RAM (8 GiB)`, with
    /// both sizes rounded down.
    pub fn percent_of(percent: u8, memory: EffectiveMemory) -> Self {
        let bytes = memory.bytes.saturating_mul(u64::from(percent)) / 100;
        let label = format!(
            "{percent}% of {} ({})",
            memory.describe(),
            format_memory(bytes, Rounding::Down)
        );
        Self { bytes, label, memory: Some(memory) }
    }

    /// The budget a `--max-ram` or `UROLLUP_MAX_RAM` value requests.
    ///
    /// A byte size is exact, labeled `explicit_label` (such as `--max-ram 6G`), and never
    /// calls `memory`. A percent takes that share of the discovered effective memory and
    /// fails when it is unknown. Either may exceed the effective memory: an explicit
    /// over-commit is honored.
    pub fn from_request(
        request: RamBudget,
        explicit_label: &str,
        memory: impl FnOnce() -> Option<EffectiveMemory>,
    ) -> Result<Self, RamBudgetError> {
        match request {
            RamBudget::Bytes(bytes) => Ok(Self::exact(bytes, explicit_label)),
            RamBudget::Percent(percent) => memory()
                .map(|memory| Self::percent_of(percent, memory))
                .ok_or(RamBudgetError::UnknownEffectiveMemory { percent }),
        }
    }

    /// The budget in bytes.
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// What the budget is, for refusals and statistics.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The effective memory `M` that a percent or the default was taken from: `None` for
    /// an explicit size, which never probes, and for the fallback, when `M` is unknown.
    /// A refusal's smallest sufficient percent is a percent of it.
    pub const fn memory(&self) -> Option<EffectiveMemory> {
        self.memory
    }
}

/// Which way [`format_memory`] rounds a size that is not a whole number of units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rounding {
    /// Toward zero, for budgets and allowances, so a printed budget never exceeds the real
    /// one.
    Down,
    /// Away from zero, for estimates and floors, so a printed estimate always exceeds a
    /// printed budget that the estimate exceeds.
    Up,
}

/// A byte count for people: whole binary units when exact, otherwise one decimal of the
/// largest unit that fits, rounded as `rounding` says, such as `8 GiB`, `9.4 GiB` or
/// `512 bytes`. A size that rounds up to 1024 of a unit prints as `1.0` of the next.
pub fn format_memory(bytes: u64, rounding: Rounding) -> String {
    const UNITS: [(u64, &str); 4] =
        [(1 << 40, "TiB"), (1 << 30, "GiB"), (1 << 20, "MiB"), (1 << 10, "KiB")];
    let mut larger = None;
    for (unit, name) in UNITS {
        if bytes < unit {
            larger = Some(name);
            continue;
        }
        if bytes % unit == 0 {
            return format!("{} {name}", bytes / unit);
        }
        let scaled = u128::from(bytes) * 10;
        let tenths = match rounding {
            Rounding::Down => scaled / u128::from(unit),
            Rounding::Up => scaled.div_ceil(u128::from(unit)),
        };
        if let (10_240, Some(larger)) = (tenths, larger) {
            return format!("1.0 {larger}");
        }
        return format!("{}.{} {name}", tenths / 10, tenths % 10);
    }
    format!("{bytes} bytes")
}

/// Physical RAM in bytes, or `None` when the host does not publish it.
pub fn physical_memory_bytes() -> Option<u64> {
    static CACHED: OnceLock<Option<u64>> = OnceLock::new();
    *CACHED.get_or_init(read_physical_memory)
}

fn read_physical_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    let bytes = linux_physical_memory();
    #[cfg(target_os = "macos")]
    let bytes = macos_physical_memory();
    #[cfg(windows)]
    let bytes = windows_physical_memory();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let bytes = None;
    bytes
}

fn parse_count(number: &str, source: &str) -> Result<u64, RamBudgetError> {
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RamBudgetError::Invalid { source: source.to_owned() });
    }
    number.parse().map_err(|_| RamBudgetError::Overflow { source: source.to_owned() })
}

fn unit_multiplier(unit: &str, source: &str) -> Result<u64, RamBudgetError> {
    let unit = unit.to_ascii_lowercase();
    Ok(match unit.as_str() {
        "" | "b" => 1,
        "k" | "kib" => 1 << 10,
        "kb" => 1_000,
        "m" | "mib" => 1 << 20,
        "mb" => 1_000_000,
        "g" | "gib" => 1 << 30,
        "gb" => 1_000_000_000,
        "t" | "tib" => 1 << 40,
        "tb" => 1_000_000_000_000,
        _ => return Err(RamBudgetError::Invalid { source: source.to_owned() }),
    })
}

fn format_byte_budget(bytes: u64) -> String {
    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;
    const GB: u64 = 1_000_000_000;
    const MB: u64 = 1_000_000;
    if bytes > 0 && bytes % GIB == 0 {
        format!("{} GiB", bytes / GIB)
    } else if bytes > 0 && bytes % MIB == 0 {
        format!("{} MiB", bytes / MIB)
    } else if bytes > 0 && bytes % GB == 0 {
        format!("{} GB", bytes / GB)
    } else if bytes > 0 && bytes % MB == 0 {
        format!("{} MB", bytes / MB)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(target_os = "linux")]
fn linux_physical_memory() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("MemTotal:") else {
            continue;
        };
        let kib = rest.split_whitespace().next()?.parse::<u64>().ok()?;
        return kib.checked_mul(1024);
    }
    None
}

// Native queries avoid invoking shells or CIM providers, which can hang or write
// profile/cache files even for a read-only usage report.
#[cfg(target_os = "macos")]
#[expect(unsafe_code, reason = "read-only sysctl with a fixed, checked output buffer")]
fn macos_physical_memory() -> Option<u64> {
    use std::ffi::{c_char, c_int, c_void};

    unsafe extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> c_int;
    }

    let mut bytes = 0_u64;
    let mut length = size_of::<u64>();
    // SAFETY: the name is NUL-terminated; both output pointers refer to live,
    // aligned, exclusively borrowed storage. The supplied length is that storage's
    // size. A null newp and zero newlen request no mutation. sysctlbyname retains
    // no pointers. The SDK declares hw.memsize as a 64-bit byte count.
    let result = unsafe {
        sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut bytes).cast(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    (result == 0 && length == size_of::<u64>() && bytes > 0).then_some(bytes)
}

#[cfg(windows)]
#[expect(unsafe_code, reason = "read-only Windows query with a fixed ABI buffer")]
fn windows_physical_memory() -> Option<u64> {
    // MEMORYSTATUSEX from the Windows SDK: two DWORDs followed by seven DWORDLONGs.
    // https://learn.microsoft.com/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex
    #[repr(C)]
    #[derive(Default)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    const _: () = assert!(size_of::<MemoryStatusEx>() == 64);

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }

    let mut status = MemoryStatusEx {
        length: u32::try_from(size_of::<MemoryStatusEx>()).ok()?,
        ..MemoryStatusEx::default()
    };
    // SAFETY: repr(C) matches the SDK layout; length describes the whole initialized,
    // aligned buffer. The API writes only that buffer and retains no pointer.
    // extern "system" supplies the Windows calling convention; BOOL is a 32-bit int.
    let result = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    (result != 0 && status.total_phys > 0).then_some(status.total_phys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_binary_sizes_si_sizes_and_percents() {
        assert_eq!(RamBudget::parse("512M").unwrap(), RamBudget::Bytes(512 << 20));
        assert_eq!(RamBudget::parse("8G").unwrap(), RamBudget::Bytes(8 << 30));
        assert_eq!(RamBudget::parse("8GiB").unwrap(), RamBudget::Bytes(8 << 30));
        assert_eq!(RamBudget::parse("8GB").unwrap(), RamBudget::Bytes(8_000_000_000));
        assert_eq!(RamBudget::parse("25%").unwrap(), RamBudget::Percent(25));
        assert_eq!(RamBudget::parse(" 25 % ").unwrap(), RamBudget::Percent(25));
        assert_eq!(RamBudget::parse("1024").unwrap(), RamBudget::Bytes(1024));
    }

    #[test]
    fn parse_rejects_empty_zero_and_unknown_units() {
        assert_eq!(RamBudget::parse("").unwrap_err(), RamBudgetError::Empty);
        assert_eq!(RamBudget::parse("0").unwrap_err(), RamBudgetError::Zero { source: "0".into() });
        assert!(matches!(RamBudget::parse("8X"), Err(RamBudgetError::Invalid { .. })));
        assert!(matches!(RamBudget::parse("0%"), Err(RamBudgetError::PercentOutOfRange { .. })));
        assert!(matches!(RamBudget::parse("101%"), Err(RamBudgetError::PercentOutOfRange { .. })));
    }

    #[test]
    fn twenty_five_percent_of_32_gib_is_8_gib_of_rows() {
        let ram = 32 << 30;
        let capacity = RamBudget::Percent(25).to_capacity(Some(ram)).unwrap();
        let expected_bytes = 8 << 30;
        assert_eq!(capacity.maximum(), rows_for_budget(expected_bytes));
        assert_eq!(capacity.label(), "25% of RAM (8 GiB)");
        assert_eq!(ObservationCapacity::default_for_ram(Some(ram)).maximum(), capacity.maximum());
    }

    #[test]
    fn unknown_ram_falls_back_to_2_gib() {
        let capacity = ObservationCapacity::default_for_ram(None);
        assert_eq!(capacity.maximum(), FALLBACK_MAX_OBSERVATIONS);
        assert_eq!(capacity.label(), "2 GiB");
        assert_eq!(capacity, ObservationCapacity::fallback());
        let ceiling = FALLBACK_BUDGET_BYTES;
        let row = u64::try_from(size_of::<RequestObservation>()).unwrap();
        let maximum = u64::try_from(capacity.maximum()).unwrap();
        assert!(maximum * row <= ceiling && (maximum + 1) * row > ceiling, "{maximum} rows");
    }

    #[test]
    fn percent_without_ram_is_an_error() {
        assert_eq!(
            RamBudget::Percent(25).to_capacity(None).unwrap_err(),
            RamBudgetError::UnknownPhysicalMemory { percent: 25 }
        );
    }

    #[test]
    fn max_rows_alone_overrides_the_default() {
        let ram = 32 << 30;
        let capacity = resolve_capacity(None, Some(10), Some(ram)).unwrap();
        assert_eq!(capacity, ObservationCapacity::from_rows(10));
    }

    #[test]
    fn both_controls_keep_the_stricter_ceiling() {
        let ram = 32 << 30;
        let loose_rows =
            resolve_capacity(Some(RamBudget::Bytes(8 << 30)), Some(10), Some(ram)).unwrap();
        assert_eq!(loose_rows, ObservationCapacity::from_rows(10));
        let tight_ram =
            resolve_capacity(Some(RamBudget::Bytes(224)), Some(10_000), Some(ram)).unwrap();
        assert_eq!(tight_ram.maximum(), rows_for_budget(224));
        assert_eq!(tight_ram.label(), "224 bytes");
    }

    #[test]
    fn fallback_row_count_tracks_the_observation_row() {
        assert_eq!(FALLBACK_MAX_OBSERVATIONS, rows_for_budget(FALLBACK_BUDGET_BYTES));
    }

    const GIB: u64 = 1 << 30;

    /// A fake `/` with the given files, relative to it.
    fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
        let files: Vec<(&str, &[u8])> =
            files.iter().map(|(path, text)| (*path, text.as_bytes())).collect();
        byte_tree(&files)
    }

    /// [`tree`] with file contents that need not be UTF-8.
    fn byte_tree(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        root
    }

    const UNIFIED_MOUNT: &str =
        "30 25 0:26 / /sys/fs/cgroup rw,nosuid,nodev shared:4 - cgroup2 cgroup2 rw,nsdelegate\n";
    const SESSION: &str = "0::/user.slice/user-1000.slice/session-3.scope\n";

    fn effective(root: &tempfile::TempDir, physical: Option<u64>) -> Option<EffectiveMemory> {
        effective_memory_under(root.path(), physical)
    }

    #[test]
    fn cgroup_v2_takes_the_smallest_limit_of_the_cgroup_and_its_ancestors() {
        let root = tree(&[
            ("proc/self/cgroup", SESSION),
            ("proc/self/mountinfo", UNIFIED_MOUNT),
            ("sys/fs/cgroup/user.slice/memory.max", "8589934592\n"),
            ("sys/fs/cgroup/user.slice/user-1000.slice/memory.max", "4294967296\n"),
            ("sys/fs/cgroup/user.slice/user-1000.slice/memory.high", "max\n"),
            ("sys/fs/cgroup/user.slice/user-1000.slice/session-3.scope/memory.max", "max\n"),
            // The mount point is the root cgroup; a value there is still read.
            ("sys/fs/cgroup/memory.max", "17179869184\n"),
        ]);
        let memory = effective(&root, Some(32 * GIB)).unwrap();
        assert_eq!(memory, EffectiveMemory { bytes: 4 * GIB, source: MemorySource::CgroupMax });
        assert_eq!(memory.describe(), "the 4 GiB cgroup limit");
        assert_eq!(
            MemoryBudget::default_for(Some(memory)).label(),
            "25% of the 4 GiB cgroup limit (1 GiB)"
        );
    }

    #[test]
    fn cgroup_v2_memory_high_counts_when_it_is_smaller() {
        let root = tree(&[
            ("proc/self/cgroup", "0::/app\n"),
            ("proc/self/mountinfo", UNIFIED_MOUNT),
            ("sys/fs/cgroup/app/memory.max", "4294967296\n"),
            ("sys/fs/cgroup/app/memory.high", "3221225472\n"),
        ]);
        let memory = effective(&root, Some(32 * GIB)).unwrap();
        assert_eq!(memory, EffectiveMemory { bytes: 3 * GIB, source: MemorySource::CgroupHigh });
        assert_eq!(memory.describe(), "the 3 GiB cgroup memory.high limit");
    }

    #[test]
    fn cgroup_v2_without_any_limit_keeps_physical_ram() {
        let root = tree(&[
            ("proc/self/cgroup", SESSION),
            ("proc/self/mountinfo", UNIFIED_MOUNT),
            ("sys/fs/cgroup/user.slice/memory.max", "max\n"),
        ]);
        let memory = effective(&root, Some(32 * GIB)).unwrap();
        assert_eq!(memory, EffectiveMemory { bytes: 32 * GIB, source: MemorySource::PhysicalRam });
        assert_eq!(
            MemoryBudget::default_for(Some(memory)).label(),
            "25% of 32 GiB physical RAM (8 GiB)"
        );
    }

    #[test]
    fn a_container_mount_root_maps_the_cgroup_path_to_the_mount_point() {
        // Without a cgroup namespace, a container sees its full path in /proc/self/cgroup
        // and the cgroup filesystem mounted from that path.
        let root = tree(&[
            ("proc/self/cgroup", "0::/docker/abc\n"),
            (
                "proc/self/mountinfo",
                "40 30 0:26 /docker/abc /sys/fs/cgroup ro - cgroup2 cgroup2 rw\n",
            ),
            ("sys/fs/cgroup/memory.max", "2147483648\n"),
        ]);
        assert_eq!(
            effective(&root, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 2 * GIB, source: MemorySource::CgroupMax })
        );
    }

    #[test]
    fn cgroup_v1_reads_the_hierarchical_limit_and_ignores_unlimited_values() {
        let files = |limit: &str| {
            tree(&[
                ("proc/self/cgroup", "5:memory:/job\n3:cpu,cpuacct:/job\n"),
                (
                    "proc/self/mountinfo",
                    "31 25 0:27 / /sys/fs/cgroup/memory rw - cgroup cgroup rw,memory\n",
                ),
                (
                    "sys/fs/cgroup/memory/job/memory.stat",
                    &format!("cache 0\nhierarchical_memory_limit {limit}\nrss 1\n"),
                ),
            ])
        };
        assert_eq!(
            effective(&files("6442450944"), Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 6 * GIB, source: MemorySource::CgroupV1Limit })
        );
        // The page-counter maximum, and anything at or above physical RAM, is no limit.
        let physical = Some(EffectiveMemory { bytes: 32 * GIB, source: MemorySource::PhysicalRam });
        assert_eq!(effective(&files("9223372036854771712"), Some(32 * GIB)), physical);
        assert_eq!(effective(&files("34359738368"), Some(32 * GIB)), physical);
        assert_eq!(effective(&files("9223372036854771712"), None), None);
    }

    #[test]
    fn soft_rlimits_bound_the_effective_memory() {
        let limits = |address: &str, data: &str| {
            tree(&[(
                "proc/self/limits",
                &format!(
                    "Limit                     Soft Limit           Hard Limit           Units     \n\
                     Max data size             {data:<20} unlimited            bytes     \n\
                     Max stack size            8388608              unlimited            bytes     \n\
                     Max address space         {address:<20} unlimited            bytes     \n"
                ),
            )])
        };
        let address = effective(&limits("8589934592", "unlimited"), Some(32 * GIB)).unwrap();
        assert_eq!(address.source, MemorySource::AddressSpaceLimit);
        assert_eq!(address.describe(), "the 8 GiB address-space limit");
        let data = effective(&limits("8589934592", "2147483648"), Some(32 * GIB)).unwrap();
        assert_eq!(data, EffectiveMemory { bytes: 2 * GIB, source: MemorySource::DataSizeLimit });
        assert_eq!(
            effective(&limits("unlimited", "unlimited"), Some(32 * GIB)).unwrap().source,
            MemorySource::PhysicalRam
        );
    }

    #[test]
    fn malformed_and_missing_files_are_ignored() {
        let root = tree(&[
            ("proc/self/cgroup", "not a cgroup line\n0::/app\n"),
            ("proc/self/mountinfo", "garbage\n30 25 0:26 / /sys/fs/cgroup - cgroup2\n"),
            ("sys/fs/cgroup/app/memory.max", "4G\n"),
            ("sys/fs/cgroup/app/memory.high", "\n"),
            ("proc/self/limits", "Max address space         -1 unlimited bytes\n"),
        ]);
        assert_eq!(
            effective(&root, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 32 * GIB, source: MemorySource::PhysicalRam })
        );
        let empty = tree(&[]);
        assert_eq!(effective(&empty, None), None);
        assert_eq!(effective(&empty, Some(GIB)).unwrap().source, MemorySource::PhysicalRam);
    }

    #[test]
    fn a_line_that_is_not_utf8_drops_only_itself() {
        let limited = EffectiveMemory { bytes: GIB, source: MemorySource::CgroupMax };
        // An unrelated mount whose point is a Latin-1 name.
        let mountinfo = byte_tree(&[
            ("proc/self/cgroup", b"0::/app\n"),
            (
                "proc/self/mountinfo",
                b"30 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n\
                  41 25 8:1 / /media/caf\xe9 rw - vfat /dev/sdb1 rw\n",
            ),
            ("sys/fs/cgroup/app/memory.max", b"1073741824\n"),
        ]);
        assert_eq!(effective(&mountinfo, Some(32 * GIB)), Some(limited));
        // A v1 named hierarchy with a Latin-1 cgroup name.
        let cgroup = byte_tree(&[
            ("proc/self/cgroup", b"1:name=systemd:/caf\xe9\n0::/app\n"),
            ("proc/self/mountinfo", b"30 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n"),
            ("sys/fs/cgroup/app/memory.max", b"1073741824\n"),
        ]);
        assert_eq!(effective(&cgroup, Some(32 * GIB)), Some(limited));
        let limits = byte_tree(&[(
            "proc/self/limits",
            b"Max stack size            \xff                    unlimited            bytes\n\
              Max address space         1073741824           unlimited            bytes\n",
        )]);
        assert_eq!(
            effective(&limits, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: GIB, source: MemorySource::AddressSpaceLimit })
        );
    }

    #[test]
    fn every_cgroup2_mount_that_shows_the_cgroup_is_read() {
        // A bind mount of a subtree listed first must not hide the ancestors' limits.
        let bind = tree(&[
            ("proc/self/cgroup", "0::/user.slice/u.slice/s.scope\n"),
            (
                "proc/self/mountinfo",
                "50 25 0:26 /user.slice/u.slice /mnt/sub rw - cgroup2 cgroup2 rw\n\
                 30 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n",
            ),
            ("sys/fs/cgroup/user.slice/memory.max", "2147483648\n"),
            ("mnt/sub/s.scope/memory.max", "max\n"),
        ]);
        assert_eq!(
            effective(&bind, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 2 * GIB, source: MemorySource::CgroupMax })
        );
        // A first mount with no readable directory contributes nothing.
        let unreadable = tree(&[
            ("proc/self/cgroup", "0::/app\n"),
            (
                "proc/self/mountinfo",
                "29 25 0:30 / /run/other rw - cgroup2 cgroup2 rw\n\
                 30 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n",
            ),
            ("sys/fs/cgroup/app/memory.max", "1073741824\n"),
        ]);
        assert_eq!(
            effective(&unreadable, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: GIB, source: MemorySource::CgroupMax })
        );
        // The smallest limit across mounts wins.
        let both = tree(&[
            ("proc/self/cgroup", "0::/app\n"),
            (
                "proc/self/mountinfo",
                "29 25 0:26 / /run/first rw - cgroup2 cgroup2 rw\n\
                 30 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n",
            ),
            ("run/first/app/memory.high", "3221225472\n"),
            ("sys/fs/cgroup/app/memory.max", "2147483648\n"),
        ]);
        assert_eq!(
            effective(&both, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 2 * GIB, source: MemorySource::CgroupMax })
        );
    }

    #[test]
    fn every_memory_v1_mount_that_shows_the_cgroup_is_read() {
        let root = tree(&[
            ("proc/self/cgroup", "5:memory:/job\n"),
            (
                "proc/self/mountinfo",
                "29 25 0:30 / /run/memory rw - cgroup cgroup rw,memory\n\
                 31 25 0:27 / /sys/fs/cgroup/memory rw - cgroup cgroup rw,memory\n",
            ),
            ("sys/fs/cgroup/memory/job/memory.stat", "hierarchical_memory_limit 6442450944\n"),
        ]);
        assert_eq!(
            effective(&root, Some(32 * GIB)),
            Some(EffectiveMemory { bytes: 6 * GIB, source: MemorySource::CgroupV1Limit })
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn effective_memory_reads_this_host_within_physical_ram() {
        // CI hosts may set cgroup limits, so the value is bounded rather than pinned.
        let physical = physical_memory_bytes().expect("Linux publishes MemTotal");
        let memory = effective_memory().expect("physical RAM is known");
        assert!(memory.bytes > 0, "{memory:?}");
        assert!(memory.bytes <= physical, "{memory:?} exceeds {physical} bytes of RAM");
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn effective_memory_is_physical_ram_off_linux() {
        assert_eq!(effective_memory(), EffectiveMemory::smallest(physical_memory_bytes(), []));
    }

    #[test]
    fn an_allowance_counts_when_physical_ram_is_unknown() {
        let root = tree(&[
            ("proc/self/cgroup", "0::/\n"),
            ("proc/self/mountinfo", UNIFIED_MOUNT),
            ("sys/fs/cgroup/memory.max", "1073741824\n"),
        ]);
        let memory = effective(&root, None).unwrap();
        assert_eq!(memory, EffectiveMemory { bytes: GIB, source: MemorySource::CgroupMax });
        assert_eq!(MemoryBudget::default_for(Some(memory)).bytes(), GIB / 4);
    }

    #[test]
    fn physical_ram_wins_a_tie_and_the_first_smallest_allowance_wins_otherwise() {
        let tie = EffectiveMemory::smallest(Some(GIB), [(GIB, MemorySource::CgroupMax)]);
        assert_eq!(tie.unwrap().source, MemorySource::PhysicalRam);
        let first = EffectiveMemory::smallest(
            None,
            [(GIB, MemorySource::CgroupHigh), (GIB, MemorySource::CgroupMax)],
        );
        assert_eq!(first.unwrap().source, MemorySource::CgroupHigh);
    }

    #[test]
    fn budgets_name_their_source_and_keep_its_memory() {
        let fallback = MemoryBudget::default_for(None);
        assert_eq!(fallback, MemoryBudget::exact(2 * GIB, "2 GiB fallback (physical RAM unknown)"));
        assert_eq!(fallback.memory(), None);
        let ram = EffectiveMemory { bytes: 32 * GIB, source: MemorySource::PhysicalRam };
        let half =
            MemoryBudget::from_request(RamBudget::Percent(50), "--max-ram 50%", || Some(ram))
                .unwrap();
        assert_eq!(half.bytes(), 16 * GIB);
        assert_eq!(half.label(), "50% of 32 GiB physical RAM (16 GiB)");
        assert_eq!(half.memory(), Some(ram));
        assert_eq!(MemoryBudget::default_for(Some(ram)).memory(), Some(ram));
        // Above the effective memory, an explicit size is still honored.
        let over =
            MemoryBudget::from_request(RamBudget::Bytes(64 * GIB), "--max-ram 64G", || Some(ram))
                .unwrap();
        assert_eq!(over, MemoryBudget::exact(64 * GIB, "--max-ram 64G"));
        assert_eq!(over.memory(), None);
        // Sizes in a label round down: 15.57 GiB and 3.89 GiB.
        let odd = EffectiveMemory { bytes: 16_715_173_888, source: MemorySource::PhysicalRam };
        assert_eq!(
            MemoryBudget::default_for(Some(odd)).label(),
            "25% of 15.5 GiB physical RAM (3.8 GiB)"
        );
    }

    #[test]
    fn an_explicit_size_never_probes_and_a_percent_needs_a_known_size() {
        let explicit =
            MemoryBudget::from_request(RamBudget::Bytes(6 * GIB), "--max-ram 6G", || {
                unreachable!("an explicit size must not probe the host")
            });
        assert_eq!(explicit.unwrap(), MemoryBudget::exact(6 * GIB, "--max-ram 6G"));
        assert_eq!(
            MemoryBudget::from_request(RamBudget::Percent(25), "--max-ram 25%", || None)
                .unwrap_err(),
            RamBudgetError::UnknownEffectiveMemory { percent: 25 }
        );
    }

    #[test]
    fn memory_sizes_print_exact_units_or_one_rounded_decimal() {
        for rounding in [Rounding::Down, Rounding::Up] {
            assert_eq!(format_memory(8 * GIB, rounding), "8 GiB");
            assert_eq!(format_memory(512 << 20, rounding), "512 MiB");
            assert_eq!(format_memory(1536, rounding), "1.5 KiB");
            assert_eq!(format_memory(512, rounding), "512 bytes");
            assert_eq!(format_memory(0, rounding), "0 bytes");
        }
        // Budgets and allowances round down; estimates and floors round up.
        assert_eq!(format_memory(10_093_173_555, Rounding::Down), "9.4 GiB");
        assert_eq!(format_memory(10_093_173_555, Rounding::Up), "9.5 GiB");
        assert_eq!(format_memory(1025, Rounding::Down), "1.0 KiB");
        assert_eq!(format_memory(1025, Rounding::Up), "1.1 KiB");
        assert_eq!(format_memory(u64::MAX, Rounding::Down), "16777215.9 TiB");
        assert_eq!(format_memory(u64::MAX, Rounding::Up), "16777216.0 TiB");
    }

    #[test]
    fn a_size_just_under_a_unit_never_prints_1024_of_the_smaller_one() {
        assert_eq!(format_memory(GIB - 1, Rounding::Down), "1023.9 MiB");
        assert_eq!(format_memory(GIB - 1, Rounding::Up), "1.0 GiB");
        assert_eq!(format_memory((1 << 20) - 1, Rounding::Up), "1.0 MiB");
        assert_eq!(format_memory((1 << 40) - 1, Rounding::Up), "1.0 TiB");
        assert_eq!(format_memory(1023, Rounding::Up), "1023 bytes");
    }

    #[test]
    fn an_estimate_one_byte_over_its_budget_prints_above_it() {
        let budget = 8 * GIB + (100 << 20);
        assert_eq!(format_memory(budget, Rounding::Down), "8.0 GiB");
        assert_eq!(format_memory(budget + 1, Rounding::Up), "8.1 GiB");
        // At a whole unit the budget prints exactly, and the estimate above it.
        assert_eq!(format_memory(8 * GIB, Rounding::Down), "8 GiB");
        assert_eq!(format_memory(8 * GIB + 1, Rounding::Up), "8.1 GiB");
    }
}

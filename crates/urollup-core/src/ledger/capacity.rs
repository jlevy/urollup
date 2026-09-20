//! Per-agent observation-row ceiling derived from a RAM budget.
//!
//! Reconciliation refuses more compact rows than this ceiling. The default is 25% of
//! physical RAM, falling back to 2 GiB when RAM cannot be read. `--max-ram` and
//! `UROLLUP_MAX_RAM` parse a byte size or a percent; `--max-rows` is an exact count.

use std::fmt;
use std::mem::size_of;
use std::sync::OnceLock;

use super::reconcile::RequestObservation;

/// Bytes used when physical RAM cannot be read, so CI and containers stay bounded.
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

#[cfg(target_os = "macos")]
fn macos_physical_memory() -> Option<u64> {
    let output = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    std::str::from_utf8(&output.stdout).ok()?.trim().parse().ok()
}

#[cfg(windows)]
fn windows_physical_memory() -> Option<u64> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance -ClassName Win32_ComputerSystem).TotalPhysicalMemory",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    std::str::from_utf8(&output.stdout).ok()?.trim().parse().ok()
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
}

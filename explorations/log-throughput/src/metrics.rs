//! Process metrics, time helpers and small numeric utilities.

use std::time::{SystemTime, UNIX_EPOCH};

pub const MIB: f64 = 1024.0 * 1024.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Usage {
    pub user_s: f64,
    pub sys_s: f64,
    pub max_rss_bytes: u64,
}

/// `getrusage(RUSAGE_SELF)`: CPU time so far and peak resident set size.
pub fn rusage() -> Usage {
    // SAFETY: `getrusage` fills the zeroed struct we pass and has no other effects.
    let ru = unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut ru);
        ru
    };
    let tv = |t: libc::timeval| t.tv_sec as f64 + t.tv_usec as f64 / 1e6;
    // macOS reports ru_maxrss in bytes, Linux in KiB.
    let rss_unit = if cfg!(target_os = "macos") { 1 } else { 1024 };
    Usage {
        user_s: tv(ru.ru_utime),
        sys_s: tv(ru.ru_stime),
        max_rss_bytes: ru.ru_maxrss as u64 * rss_unit,
    }
}

/// One-minute load average, to record contention from other processes.
pub fn load1() -> f64 {
    let mut avg = [0f64; 3];
    // SAFETY: `getloadavg` writes at most 3 doubles into the array.
    let n = unsafe { libc::getloadavg(avg.as_mut_ptr(), 3) };
    if n >= 1 { avg[0] } else { -1.0 }
}

pub fn now_epoch() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// UTC calendar date of a Unix time as `yyyymmdd`.
pub fn epoch_to_day(epoch: i64) -> u32 {
    // Howard Hinnant's civil_from_days.
    let z = epoch.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y * 10_000 + m * 100 + d) as u32
}

/// Parses the `YYYY-MM-DD` prefix of an ISO timestamp into `yyyymmdd`; 0 if absent.
pub fn iso_day(s: &[u8]) -> u32 {
    if s.len() < 10 || s[4] != b'-' || s[7] != b'-' {
        return 0;
    }
    let mut out = 0u32;
    for (i, &b) in s[..10].iter().enumerate() {
        if i == 4 || i == 7 {
            continue;
        }
        if !b.is_ascii_digit() {
            return 0;
        }
        out = out * 10 + u32::from(b - b'0');
    }
    out
}

/// Nearest-rank percentile of an ascending slice.
pub fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

pub fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

pub fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_timestamps() {
        assert_eq!(epoch_to_day(0), 19_700_101);
        assert_eq!(epoch_to_day(1_789_430_400), 20_260_915);
        assert_eq!(epoch_to_day(1_789_430_399), 20_260_914);
        assert_eq!(iso_day(b"2026-09-14T16:16:48.211Z"), 20_260_914);
        assert_eq!(iso_day(b"not a date"), 0);
    }

    #[test]
    fn percentiles_and_median() {
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 50.0), 50);
        assert_eq!(percentile(&v, 99.0), 99);
        assert_eq!(percentile(&v, 100.0), 100);
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
    }
}

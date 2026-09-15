//! Streaming complete-line reads over plain, gzip and zstd sources.

use std::fs::File;
use std::io::{self, BufReader, ErrorKind, Read};
use std::path::Path;

use crate::manifest::{Comp, Entry};

pub const CHUNK: usize = 1 << 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineStats {
    /// Decompressed bytes read, including any pending tail.
    pub bytes: u64,
    /// Complete (newline-terminated) lines.
    pub lines: u64,
    /// Bytes after the last newline: an unfinished record, never parsed.
    pub pending_bytes: u64,
    pub max_line: u64,
}

/// Opens a manifest entry read-only, limited to the byte extent frozen in the manifest.
pub fn open_entry(entry: &Entry) -> io::Result<Box<dyn Read + Send>> {
    let file = File::open(&entry.path)?.take(entry.size);
    Ok(match entry.comp {
        Comp::Plain => Box::new(file),
        Comp::Gzip => {
            Box::new(flate2::read::MultiGzDecoder::new(BufReader::with_capacity(CHUNK, file)))
        }
        Comp::Zstd => Box::new(zstd::stream::read::Decoder::new(file)?),
    })
}

pub fn open_zstd(path: &Path) -> io::Result<Box<dyn Read + Send>> {
    Ok(Box::new(zstd::stream::read::Decoder::new(File::open(path)?)?))
}

fn read_some<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        match r.read(buf) {
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            other => return other,
        }
    }
}

/// Calls `f` on every complete line (without its newline), copying only partial lines
/// that straddle chunk boundaries.
pub fn for_each_line<R: Read>(mut r: R, mut f: impl FnMut(&[u8])) -> io::Result<LineStats> {
    let mut buf = vec![0u8; CHUNK];
    let mut len = 0usize;
    let mut stats = LineStats::default();
    loop {
        if len == buf.len() {
            buf.resize(buf.len() * 2, 0);
        }
        let n = read_some(&mut r, &mut buf[len..])?;
        if n == 0 {
            break;
        }
        stats.bytes += n as u64;
        let scan_from = len;
        len += n;
        let mut start = 0usize;
        for pos in memchr::memchr_iter(b'\n', &buf[scan_from..len]) {
            let end = scan_from + pos;
            stats.lines += 1;
            stats.max_line = stats.max_line.max((end - start) as u64);
            f(&buf[start..end]);
            start = end + 1;
        }
        if start > 0 {
            buf.copy_within(start..len, 0);
            len -= start;
        }
    }
    stats.pending_bytes = len as u64;
    Ok(stats)
}

/// Reads and discards all bytes, counting newlines: the I/O and decompression floor.
pub fn read_only<R: Read>(mut r: R) -> io::Result<LineStats> {
    let mut buf = vec![0u8; CHUNK];
    let mut stats = LineStats::default();
    let mut since_newline = 0u64;
    loop {
        let n = read_some(&mut r, &mut buf)?;
        if n == 0 {
            break;
        }
        stats.bytes += n as u64;
        let chunk = &buf[..n];
        let mut count = 0u64;
        let mut last = None;
        for pos in memchr::memchr_iter(b'\n', chunk) {
            count += 1;
            last = Some(pos);
        }
        stats.lines += count;
        since_newline = match last {
            Some(pos) => (n - pos - 1) as u64,
            None => since_newline + n as u64,
        };
    }
    stats.pending_bytes = since_newline;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that returns at most `step` bytes per call, to exercise chunk boundaries.
    struct Trickle<'a> {
        data: &'a [u8],
        step: usize,
    }

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.step.min(buf.len()).min(self.data.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.data = &self.data[n..];
            Ok(n)
        }
    }

    #[test]
    fn lines_across_chunks_with_pending_tail() {
        let long = "x".repeat(3 * CHUNK);
        let data = format!("a\n{long}\n\nlast-without-newline");
        let mut seen = Vec::new();
        let stats =
            for_each_line(Trickle { data: data.as_bytes(), step: 7919 }, |l| seen.push(l.len()))
                .unwrap();
        assert_eq!(seen, vec![1, 3 * CHUNK, 0]);
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.pending_bytes, "last-without-newline".len() as u64);
        assert_eq!(stats.bytes, data.len() as u64);
        let raw = read_only(Trickle { data: data.as_bytes(), step: 7919 }).unwrap();
        assert_eq!(
            (raw.lines, raw.pending_bytes, raw.bytes),
            (3, stats.pending_bytes, stats.bytes)
        );
    }
}

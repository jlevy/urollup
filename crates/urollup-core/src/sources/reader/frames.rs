//! zstd decoding through the reader's own frame loop (scalable-ingestion plan, "zstd
//! frames").
//!
//! [`ZstdFrames`] drives zstd's streaming decoder (`zstd::stream::raw::Decoder`) over its
//! own input buffer of `ZSTD_DStreamInSize()` bytes, as `zstd::stream::read::Decoder`
//! does, and reads every frame's header before the decoder sees the frame:
//!
//! - A window of up to [`SLOT_ZSTD_WINDOW`] (8 MiB) decodes in the worker slot's decoder,
//!   opened with `window_log_max(23)`.
//! - A window above 8 MiB and up to [`ZSTD_DEFAULT_WINDOW_LIMIT`] (2^27) takes the
//!   large-record permit for that window's decoder first, and decodes that one frame with
//!   a second decoder opened with `window_log_max(27)`, dropped with its permit share when
//!   the frame ends.
//! - Anything else goes to the slot's decoder unchanged: a window above 2^27, which fails
//!   there as it did under zstd's default limit, with the same kind and message; a
//!   skippable frame, which the decoder skips; and a header the loop cannot parse (a bad
//!   magic number, a reserved bit, or a header cut off at the end of the file), so its
//!   failure stays what it was.
//!
//! A wide frame is detected before it decodes, so nothing is read twice. Apart from the
//! routing, the loop is `zstd::stream::read::Decoder`'s: the same input buffer size and
//! reads, the same chunks of input offered to the decoder, the same flush before each
//! read, the same session reset between frames, and the same `incomplete frame` error when
//! the input ends inside a frame. When the buffer ends inside a frame header, the loop
//! reads the next buffer before offering the header's start, and offers that start as a
//! chunk of its own, as `BufReader` would have; a look-ahead read that finds the end of
//! the input ends it, as `BufReader`'s would have. A read that fails, `Interrupted`
//! included, changes nothing, as `BufReader`'s does, so a caller that retries resumes
//! exactly where it stopped.
//!
//! Its output bytes, and the kind and message of any error, are that decoder's, whatever
//! the reads return; the tests compare the two over scripted readers. The one exception
//! is a read that returns fewer bytes than asked before the end of the input and leaves a
//! frame header incomplete: the loop routes that frame by the bytes it has, so a window
//! above 8 MiB goes to the slot's decoder and fails there. A local regular file returns
//! a short read only at its end, but a network or FUSE file system, or another reader
//! [`decode`](super::decode) is given, may not.

use std::io::{self, Read};

use zstd::stream::raw::{DParameter, Decoder, InBuffer, Operation, OutBuffer};

use super::stopped;
use crate::ledger::admission::model::{SLOT_ZSTD_WINDOW, ZSTD_DEFAULT_WINDOW_LIMIT, zstd_stream};
use crate::ledger::admission::{LargeRecord, MemoryAdmission};

/// `window_log_max` of the slot's decoder: windows up to 8 MiB.
const SLOT_WINDOW_LOG: u32 = 23;

/// `window_log_max` of a wide frame's decoder: zstd's default limit.
const WIDE_WINDOW_LOG: u32 = 27;

const _: () = assert!(1_u64 << SLOT_WINDOW_LOG == SLOT_ZSTD_WINDOW);
const _: () = assert!(1_u64 << WIDE_WINDOW_LOG == ZSTD_DEFAULT_WINDOW_LIMIT);

/// The magic number that opens a zstd frame (RFC 8878 §3.1.1).
const FRAME_MAGIC: u32 = 0xFD2F_B528;

/// The smallest log2 window a frame can declare, `ZSTD_WINDOWLOG_ABSOLUTEMIN`.
const MIN_WINDOW_LOG: u32 = 10;

/// What the start of a frame says about its window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FrameHeader {
    /// A zstd frame whose window is this many bytes.
    Window(u64),
    /// More bytes are needed to read the window: at least this many in all.
    Incomplete(usize),
    /// Not a zstd frame this loop routes: a skippable frame, an unknown magic number or a
    /// reserved bit. The decoder handles it as it always did.
    Other,
}

/// Reads the window of the frame that `bytes` begins with (RFC 8878 §3.1.1.1).
///
/// A single-segment frame's window is its content size; otherwise the window descriptor
/// gives `2^(10 + exponent)` plus `mantissa` eighths of it, so a 9 MiB window is not read
/// as 8 MiB. This is `ZSTD_getFrameHeader`'s rule.
pub(super) fn frame_header(bytes: &[u8]) -> FrameHeader {
    let Some(magic) = bytes.first_chunk::<4>() else {
        return FrameHeader::Incomplete(5);
    };
    if u32::from_le_bytes(*magic) != FRAME_MAGIC {
        return FrameHeader::Other;
    }
    let Some(&descriptor) = bytes.get(4) else {
        return FrameHeader::Incomplete(5);
    };
    if descriptor & 0x08 != 0 {
        return FrameHeader::Other;
    }
    let single_segment = descriptor & 0x20 != 0;
    let content_size_flag = descriptor >> 6;
    let dictionary_bytes = [0, 1, 2, 4][usize::from(descriptor & 0x03)];
    let content_bytes = match (content_size_flag, single_segment) {
        (0, false) => 0,
        (0, true) => 1,
        (1, _) => 2,
        (2, _) => 4,
        _ => 8,
    };
    let window_bytes = usize::from(!single_segment);
    let header = 5 + window_bytes + dictionary_bytes + content_bytes;
    let Some(fields) = bytes.get(5..header) else {
        return FrameHeader::Incomplete(header);
    };
    if !single_segment {
        let Some(&window) = fields.first() else { return FrameHeader::Incomplete(header) };
        let log = MIN_WINDOW_LOG + u32::from(window >> 3);
        let base = 1_u64.checked_shl(log).unwrap_or(u64::MAX);
        return FrameHeader::Window(base.saturating_add((base >> 3) * u64::from(window & 7)));
    }
    let Some(content) = fields.get(dictionary_bytes..) else {
        return FrameHeader::Incomplete(header);
    };
    let mut size = [0_u8; 8];
    for (slot, byte) in size.iter_mut().zip(content) {
        *slot = *byte;
    }
    let size = u64::from_le_bytes(size);
    FrameHeader::Window(if content_size_flag == 1 { size.saturating_add(256) } else { size })
}

/// The decoder a frame is routed to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Route {
    /// The worker slot's decoder, for windows up to 8 MiB and every frame the loop does
    /// not route by its window.
    Slot,
    /// A second decoder under the large-record permit, for a window of this many bytes.
    Wide(u64),
}

impl Route {
    /// Where a frame starting with `header` decodes.
    pub(super) const fn of(header: FrameHeader) -> Self {
        match header {
            FrameHeader::Window(window)
                if window > SLOT_ZSTD_WINDOW && window <= ZSTD_DEFAULT_WINDOW_LIMIT =>
            {
                Self::Wide(window)
            }
            FrameHeader::Window(_) | FrameHeader::Incomplete(_) | FrameHeader::Other => Self::Slot,
        }
    }
}

/// Where the loop is, as in `zstd::stream::read::Decoder`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    /// Reading compressed input.
    Reading,
    /// The input has ended.
    PastEof,
    /// The stream ended cleanly; every read returns 0.
    Finished,
}

/// Where the input is relative to its frames, as the stock reader tracks it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Frame {
    /// No frame has started: the next input byte starts the first, unrouted.
    First,
    /// Inside a routed frame.
    Inside,
    /// The last frame decoded and flushed completely, so the input may end here; the next
    /// input byte starts another frame, unrouted, and the slot's decoder resets for it.
    Between,
}

/// One wide frame's decoder and the permit share that pays for it.
struct WideFrame<'a> {
    decoder: Decoder<'static>,
    _permit: LargeRecord<'a>,
}

/// The longest zstd frame header, `ZSTD_FRAMEHEADERSIZE_MAX`.
const MAX_HEADER: usize = 18;

/// zstd decoding of `stored` through the frame loop; see the module documentation.
pub(super) struct ZstdFrames<'a, R> {
    stored: R,
    admission: &'a MemoryAdmission,
    /// The compressed input buffer; `input[start..end]` is not offered yet.
    input: Box<[u8]>,
    start: usize,
    end: usize,
    /// The start of a frame header the previous buffer ended inside: `head[head_start..
    /// head_end]` is offered before `input`, as a chunk of its own.
    head: [u8; MAX_HEADER],
    head_start: usize,
    head_end: usize,
    /// Whether a look-ahead read found the end of `stored`, which `BufReader` would have
    /// found on the read after the header's start, so no later read is made.
    ended: bool,
    /// The slot's decoder, for windows up to 8 MiB.
    slot: Decoder<'static>,
    /// The current frame's decoder, when its window needs the permit.
    wide: Option<WideFrame<'a>>,
    state: State,
    /// Where the input is relative to its frames.
    frame: Frame,
}

impl<'a, R: Read> ZstdFrames<'a, R> {
    /// The loop over `stored`, with the slot's decoder opened.
    pub(super) fn new(stored: R, admission: &'a MemoryAdmission) -> io::Result<Self> {
        Ok(Self {
            stored,
            admission,
            // `ZSTD_DStreamInSize()`, which the model charges as `ZSTD_INPUT_BUFFER`.
            input: vec![0; zstd::zstd_safe::DCtx::in_size()].into_boxed_slice(),
            start: 0,
            end: 0,
            head: [0; MAX_HEADER],
            head_start: 0,
            head_end: 0,
            ended: false,
            slot: decoder(SLOT_WINDOW_LOG)?,
            wide: None,
            state: State::Reading,
            frame: Frame::First,
        })
    }

    /// How many input bytes are ready to offer, reading the next buffer from `stored` when
    /// none are, as `BufReader` fills. A read that fails changes nothing.
    fn fill(&mut self) -> io::Result<usize> {
        if self.head_start < self.head_end {
            return Ok(self.head_end - self.head_start);
        }
        if self.start == self.end && !self.ended {
            let read = self.stored.read(&mut self.input)?;
            (self.start, self.end) = (0, read);
        }
        Ok(self.end - self.start)
    }

    /// The header of the frame the ready input starts with: the stashed header start, if
    /// any, followed by the input buffer.
    fn header(&self) -> FrameHeader {
        let head = &self.head[self.head_start..self.head_end];
        let input = &self.input[self.start..self.end];
        let more = input.len().min(MAX_HEADER);
        let mut joined = [0_u8; 2 * MAX_HEADER];
        joined[..head.len()].copy_from_slice(head);
        joined[head.len()..head.len() + more].copy_from_slice(&input[..more]);
        frame_header(&joined[..head.len() + more])
    }

    /// Moves the start of a header the input buffer ends inside to `head` and reads the
    /// next buffer, the read `BufReader` would have made once the decoder had consumed that
    /// start. A failed read restores the input as it was, so a retry repeats the look-ahead;
    /// a read of nothing is the end of `stored`.
    fn look_ahead(&mut self) -> io::Result<()> {
        // An incomplete header is shorter than the longest one.
        let length = (self.end - self.start).min(MAX_HEADER);
        let mut head = [0_u8; MAX_HEADER];
        head[..length].copy_from_slice(&self.input[self.start..self.start + length]);
        match self.stored.read(&mut self.input) {
            Ok(read) => {
                self.head = head;
                (self.head_start, self.head_end) = (0, length);
                (self.start, self.end) = (0, read);
                self.ended = read == 0;
                Ok(())
            }
            Err(error) => {
                // A failed read may still have written to the buffer.
                self.input[self.start..self.start + length].copy_from_slice(&head[..length]);
                Err(error)
            }
        }
    }

    /// Routes the frame the ready input starts with, and prepares its decoder.
    ///
    /// When the buffer ends inside the frame's header, [`Self::look_ahead`] reads the
    /// next buffer first, so the header is read whole and the decoder is still offered the
    /// same chunks. Nothing about the frame changes until it is routed, so a failure here
    /// leaves the loop to route it again.
    fn begin_frame(&mut self) -> io::Result<()> {
        if self.head_start == self.head_end && matches!(self.header(), FrameHeader::Incomplete(_)) {
            self.look_ahead()?;
        }
        let wide = match Route::of(self.header()) {
            Route::Wide(window) => {
                let permit =
                    self.admission.large_record(zstd_stream(window)).ok_or_else(stopped)?;
                Some(WideFrame { decoder: decoder(WIDE_WINDOW_LOG)?, _permit: permit })
            }
            Route::Slot => {
                if self.frame == Frame::Between {
                    self.slot.reinit()?;
                }
                None
            }
        };
        self.wide = wide;
        self.frame = Frame::Inside;
        Ok(())
    }
}

/// A streaming decoder that accepts windows up to `2^window_log`.
fn decoder(window_log: u32) -> io::Result<Decoder<'static>> {
    let mut decoder = Decoder::new()?;
    decoder.set_parameter(DParameter::WindowLogMax(window_log))?;
    Ok(decoder)
}

impl<R: Read> Read for ZstdFrames<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // The first pass offers no input, which flushes what the decoder holds back.
        let mut first = true;
        loop {
            match self.state {
                State::Reading => {
                    let available = if first { 0 } else { self.fill()? };
                    if !first && available == 0 {
                        self.state = State::PastEof;
                        continue;
                    }
                    if available > 0 && self.frame != Frame::Inside {
                        self.begin_frame()?;
                    }
                    let from_head = self.head_start < self.head_end;
                    let input = match (first, from_head) {
                        (true, _) => &[][..],
                        (false, true) => &self.head[self.head_start..self.head_end],
                        (false, false) => &self.input[self.start..self.end],
                    };
                    first = false;
                    let mut source = InBuffer::around(input);
                    let mut target = OutBuffer::around(&mut *buf);
                    let decoder = match &mut self.wide {
                        Some(wide) => &mut wide.decoder,
                        None => &mut self.slot,
                    };
                    let hint = decoder.run(&mut source, &mut target)?;
                    let (consumed, written) = (source.pos(), target.pos());
                    if from_head {
                        self.head_start += consumed;
                    } else {
                        self.start += consumed;
                    }
                    if hint == 0 {
                        // The frame is decoded and flushed; a wide frame's decoder and
                        // its permit share go with it.
                        self.frame = Frame::Between;
                        self.wide = None;
                    }
                    if written > 0 {
                        return Ok(written);
                    }
                }
                State::PastEof => {
                    if self.frame != Frame::Between {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "incomplete frame",
                        ));
                    }
                    self.state = State::Finished;
                    return Ok(0);
                }
                State::Finished => return Ok(0),
            }
        }
    }
}

#[cfg(test)]
mod tests;

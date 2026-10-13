use std::io::{self, BufRead, BufReader, Write as _};

use super::{FrameHeader, MAX_HEADER, Route, frame_header};
use crate::ledger::admission::model::{
    SLOT_ZSTD_WINDOW, ZSTD_DEFAULT_WINDOW_LIMIT, ZSTD_INPUT_BUFFER, zstd_stream,
};
use crate::ledger::admission::{Component, MemoryAdmission};
use crate::sources::manifest::Representation;
use crate::sources::reader::{READ_BUFFER_BYTES, decode};

const MIB: u64 = 1 << 20;

/// What a decoder yields: every byte before its first error, and that error's kind and
/// message.
type Outcome = (Vec<u8>, Option<(io::ErrorKind, String)>);

fn drain(reader: &mut dyn BufRead) -> Outcome {
    let mut decoded = Vec::new();
    loop {
        match reader.fill_buf() {
            Ok([]) => return (decoded, None),
            Ok(chunk) => {
                let length = chunk.len();
                decoded.extend_from_slice(chunk);
                reader.consume(length);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return (decoded, Some((error.kind(), error.to_string()))),
        }
    }
}

/// zstd's own reader behind the reader's read buffer: how the reader decoded zstd before
/// the frame loop.
fn before(bytes: &[u8]) -> Outcome {
    let decoder = zstd::stream::read::Decoder::new(bytes).unwrap();
    drain(&mut BufReader::with_capacity(READ_BUFFER_BYTES, decoder))
}

/// The reader's decoder, through the frame loop.
fn after(bytes: &[u8], admission: &MemoryAdmission) -> Outcome {
    drain(&mut *decode(bytes, Representation::Zstd, admission).unwrap())
}

/// Asserts that the frame loop decodes `bytes` exactly as zstd's reader did, and returns
/// the permit's charge for it.
fn same_as_before(bytes: &[u8], case: &str) -> u64 {
    let admission = MemoryAdmission::unlimited();
    let ours = after(bytes, &admission);
    let theirs = before(bytes);
    assert_eq!(ours.1, theirs.1, "{case}: the error");
    assert!(ours.0 == theirs.0, "{case}: {} bytes against {}", ours.0.len(), theirs.0.len());
    admission.charged_by(Component::Slots)
}

/// Lines of JSON that compress well, `count` of them.
fn records(count: usize) -> Vec<u8> {
    (0..count)
        .flat_map(|index| format!("{{\"i\":{index},\"pad\":\"xyzzy\"}}\n").into_bytes())
        .collect()
}

/// Bytes that do not compress, from a fixed xorshift stream.
fn noise(length: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

/// A multi-segment frame of `content` with a 1 KiB window and its content size, whose
/// window descriptor sits at byte 5.
fn frame(content: &[u8]) -> Vec<u8> {
    assert!(content.len() > 1024, "a frame no larger than its window is single-segment");
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
    encoder.window_log(10).unwrap();
    encoder.include_contentsize(true).unwrap();
    encoder.set_pledged_src_size(Some(content.len() as u64)).unwrap();
    encoder.write_all(content).unwrap();
    let frame = encoder.finish().unwrap();
    assert_eq!(frame_header(&frame), FrameHeader::Window(1024));
    frame
}

/// `frame` declaring the window `2^(10 + exponent)` plus `mantissa` eighths of it. A
/// larger declared window never makes a valid frame invalid.
fn widened(mut frame: Vec<u8>, exponent: u8, mantissa: u8) -> Vec<u8> {
    frame[5] = (exponent << 3) | mantissa;
    frame
}

/// A skippable frame of `length` bytes in all.
fn skippable(length: usize) -> Vec<u8> {
    let payload = u32::try_from(length - 8).unwrap();
    [&0x184D_2A53_u32.to_le_bytes()[..], &payload.to_le_bytes(), &vec![b's'; length - 8]].concat()
}

#[test]
fn frame_headers_give_the_window_zstd_reads() {
    let magic = 0xFD2F_B528_u32.to_le_bytes();
    let header = |descriptor: u8, rest: &[u8]| [&magic[..], &[descriptor], rest].concat();
    // Exponent 13 and mantissa 1: 8 MiB plus an eighth of it, not 8 MiB.
    assert_eq!(frame_header(&header(0x00, &[13 << 3 | 1])), FrameHeader::Window(9 * MIB));
    assert_eq!(frame_header(&header(0x00, &[0])), FrameHeader::Window(1024));
    assert_eq!(frame_header(&header(0x00, &[17 << 3])), FrameHeader::Window(1 << 27));
    assert_eq!(frame_header(&header(0x00, &[31 << 3 | 7])), FrameHeader::Window(15 << 38));
    // A single-segment frame's window is its content size, after any dictionary ID.
    assert_eq!(frame_header(&header(0x20, &[200])), FrameHeader::Window(200));
    assert_eq!(frame_header(&header(0x61, &[9, 1, 0])), FrameHeader::Window(256 + 1));
    assert_eq!(frame_header(&header(0xA2, &[1, 2, 0, 0, 0x90, 0])), FrameHeader::Window(0x90_0000));
    assert_eq!(
        frame_header(&header(0xE3, &[1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 1])),
        FrameHeader::Window(1 << 56)
    );
    // A content size beside a window descriptor does not change the window.
    assert_eq!(frame_header(&header(0x40, &[3 << 3, 0, 0])), FrameHeader::Window(8 * 1024));
    // Too few bytes to read the window yet.
    assert_eq!(frame_header(&magic[..3]), FrameHeader::Incomplete(5));
    assert_eq!(frame_header(&magic), FrameHeader::Incomplete(5));
    assert_eq!(frame_header(&header(0x00, &[])), FrameHeader::Incomplete(6));
    assert_eq!(frame_header(&header(0xE3, &[1, 2, 3, 4, 0])), FrameHeader::Incomplete(17));
    assert_eq!(frame_header(&header(0xE3, &[1, 2, 3, 4])), FrameHeader::Incomplete(17));
    // The longest header: content size, dictionary ID and window descriptor.
    assert_eq!(frame_header(&header(0xC3, &[0; 13])), FrameHeader::Window(1024));
    assert_eq!(
        frame_header(&header(0xC3, &[0; 13])[..MAX_HEADER - 1]),
        FrameHeader::Incomplete(18)
    );
    // Not frames the loop routes: a reserved bit, another magic number, a skippable frame.
    assert_eq!(frame_header(&header(0x08, &[0])), FrameHeader::Other);
    assert_eq!(frame_header(b"{\"a\":1}\n"), FrameHeader::Other);
    assert_eq!(frame_header(&skippable(16)), FrameHeader::Other);
}

#[test]
fn only_windows_past_the_slot_and_within_zstds_limit_take_the_permit() {
    assert_eq!(Route::of(FrameHeader::Window(SLOT_ZSTD_WINDOW)), Route::Slot);
    assert_eq!(Route::of(FrameHeader::Window(9 * MIB)), Route::Wide(9 * MIB));
    assert_eq!(
        Route::of(FrameHeader::Window(ZSTD_DEFAULT_WINDOW_LIMIT)),
        Route::Wide(ZSTD_DEFAULT_WINDOW_LIMIT)
    );
    assert_eq!(Route::of(FrameHeader::Window(ZSTD_DEFAULT_WINDOW_LIMIT + 1)), Route::Slot);
    assert_eq!(Route::of(FrameHeader::Incomplete(6)), Route::Slot);
    assert_eq!(Route::of(FrameHeader::Other), Route::Slot);
    assert_eq!(zstd::zstd_safe::DCtx::in_size() as u64, ZSTD_INPUT_BUFFER);
}

#[test]
fn every_window_from_one_kib_to_zstds_limit_decodes_as_zstds_reader_does() {
    let content = records(400);
    let small = frame(&content);
    for exponent in 0..=17_u8 {
        for mantissa in [0, 1, 7] {
            let window = (1_u64 << (10 + exponent))
                + ((1_u64 << (10 + exponent)) >> 3) * u64::from(mantissa);
            let bytes = widened(small.clone(), exponent, mantissa);
            let case = format!("multi-segment window {window}");
            let charge = same_as_before(&bytes, &case);
            let (decoded, error) = after(&bytes, &MemoryAdmission::unlimited());
            if window <= ZSTD_DEFAULT_WINDOW_LIMIT {
                assert_eq!((decoded, error), (content.clone(), None), "{case}");
            }
            let expected = match Route::of(FrameHeader::Window(window)) {
                Route::Wide(window) => zstd_stream(window),
                Route::Slot => 0,
            };
            assert_eq!(charge, expected, "{case}: the permit's charge");
        }
    }
    // Single-segment frames, whose window is their content size, on both sides of 8 MiB.
    for lines in [8, 30_000, 320_000] {
        let content = records(lines);
        let mut compressor = zstd::bulk::Compressor::new(3).unwrap();
        compressor.set_parameter(zstd::stream::raw::CParameter::WindowLog(24)).unwrap();
        let bytes = compressor.compress(&content).unwrap();
        let FrameHeader::Window(window) = frame_header(&bytes) else {
            unreachable!("a one-shot frame has a header")
        };
        assert_eq!(window, content.len() as u64, "a one-shot frame is single-segment");
        let charge = same_as_before(&bytes, &format!("single-segment {window}"));
        assert_eq!(charge > 0, window > SLOT_ZSTD_WINDOW, "single-segment {window}");
        assert_eq!(after(&bytes, &MemoryAdmission::unlimited()), (content, None));
    }
}

#[test]
fn a_window_past_zstds_limit_behaves_as_it_did() {
    // 2^28 without a content size: corrupt compressed data with zstd's own message, from
    // the slot's decoder, and no permit.
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap();
    encoder.window_log(10).unwrap();
    encoder.include_contentsize(false).unwrap();
    encoder.write_all(&records(400)).unwrap();
    let bytes = widened(encoder.finish().unwrap(), 18, 0);
    assert_eq!(frame_header(&bytes), FrameHeader::Window(1 << 28));
    assert_eq!(same_as_before(&bytes, "2^28 window"), 0);
    let (decoded, error) = after(&bytes, &MemoryAdmission::unlimited());
    assert!(decoded.is_empty());
    assert_eq!(
        error,
        Some((io::ErrorKind::Other, "Frame requires too much memory for decoding".to_owned()))
    );
    // With a content size that fits the read buffer, zstd decodes the frame in one pass
    // without its window, under any window limit, so it decodes as it did.
    let content = records(400);
    let sized = widened(frame(&content), 18, 0);
    assert_eq!(same_as_before(&sized, "2^28 window, sized"), 0);
    assert_eq!(after(&sized, &MemoryAdmission::unlimited()), (content, None));
}

#[test]
fn a_nine_mib_descriptor_takes_the_permit_for_that_frame_only() {
    let first = records(400);
    let second = records(10_000);
    let third = records(600);
    let bytes =
        [frame(&first), widened(frame(&second), 13, 1), skippable(40), frame(&third)].concat();
    let admission = MemoryAdmission::unlimited();
    let me = std::thread::current().id();
    let mut reader = decode(&bytes[..], Representation::Zstd, &admission).unwrap();
    let mut decoded = Vec::new();
    let mut held = Vec::new();
    loop {
        let chunk = reader.fill_buf().unwrap();
        if chunk.is_empty() {
            break;
        }
        let length = chunk.len();
        decoded.extend_from_slice(chunk);
        reader.consume(length);
        held.push((decoded.len(), admission.permit_holder() == Some(me)));
    }
    assert_eq!(decoded, [first.clone(), second.clone(), third].concat());
    assert_eq!(admission.charged_by(Component::Slots), zstd_stream(9 * MIB));
    // The permit is held while the wide frame decodes, and released when it ends.
    assert!(held.iter().any(|(_, held)| *held));
    assert_eq!(admission.permit_holder(), None);
    drop(reader);
}

/// The offset of each block header in the frame that `frame` starts with, whose header is
/// `header` bytes long.
fn block_headers(frame: &[u8], header: usize) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut at = header;
    while let Some(bytes) = frame.get(at..at + 3) {
        offsets.push(at);
        let word = u32::from(bytes[0]) | u32::from(bytes[1]) << 8 | u32::from(bytes[2]) << 16;
        let (last, kind, size) = (word & 1, (word >> 1) & 3, (word >> 3) as usize);
        // A run-length block stores one byte; the others store `size` bytes.
        at += 3 + if kind == 1 { 1 } else { size };
        if last == 1 {
            break;
        }
    }
    offsets
}

/// A frame of noise in raw blocks of 128 KiB with a 128 KiB window, the same frame
/// declaring a 16 MiB window, and its block headers' offsets. Raw blocks fill the input
/// buffer about one block at a time, so how much a decoder emits before a damaged block
/// depends on the chunks of input it is offered.
fn raw_block_frames() -> (Vec<u8>, Vec<u8>, Vec<usize>) {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 1).unwrap();
    encoder.window_log(17).unwrap();
    encoder.write_all(&noise(1_200_000, 3)).unwrap();
    let narrow = encoder.finish().unwrap();
    // Without a content size the header is the magic number, descriptor and window.
    assert_eq!(frame_header(&narrow), FrameHeader::Window(1 << 17));
    let headers = block_headers(&narrow, 6);
    assert!(headers.len() > 8, "{} blocks", headers.len());
    (widened(narrow.clone(), 7, 0), widened(narrow, 14, 0), headers)
}

#[test]
fn a_damaged_block_fails_after_the_same_output_as_before() {
    let (narrow, wide, headers) = raw_block_frames();
    for (name, frame) in [("narrow", narrow), ("wide", wide)] {
        for lead in [0, 5_000, 70_001] {
            let lead = if lead == 0 { Vec::new() } else { skippable(lead) };
            for header in &headers[2..] {
                for damage in [0b110, 0xF8] {
                    let mut damaged = [&lead[..], &frame[..]].concat();
                    damaged[lead.len() + header] |= damage;
                    let case =
                        format!("{name}, lead {}, block at {header} with {damage:#x}", lead.len());
                    same_as_before(&damaged, &case);
                }
            }
        }
    }
}

#[test]
fn damaged_streams_fail_with_the_kind_and_message_they_did() {
    let content = records(2_000);
    let narrow = frame(&content);
    let wide = widened(frame(&records(1_500)), 14, 0);
    let stream = [narrow.clone(), skippable(24), wide.clone(), narrow].concat();
    same_as_before(&stream, "the whole stream");
    // Every prefix: a header or frame cut off at the end of the file.
    for cut in 0..stream.len() {
        same_as_before(&stream[..cut], &format!("cut at {cut}"));
    }
    // A bad magic number, a reserved bit, garbage after a frame, and flipped bytes.
    let mut magic = stream.clone();
    magic[0] ^= 0x01;
    same_as_before(&magic, "bad magic");
    let mut reserved = wide.clone();
    reserved[4] |= 0x08;
    same_as_before(&[&stream[..], &reserved[..]].concat(), "reserved bit");
    same_as_before(&[&stream[..], b"tail"].concat(), "garbage after the last frame");
    for position in (0..stream.len()).step_by(7) {
        let mut flipped = stream.clone();
        flipped[position] ^= 0x5A;
        same_as_before(&flipped, &format!("flip at {position}"));
    }
    same_as_before(b"", "an empty file");
}

#[test]
fn a_header_split_across_input_buffers_is_offered_in_the_same_chunks() {
    let input = usize::try_from(ZSTD_INPUT_BUFFER).unwrap();
    let narrow = frame(&noise(60_000, 7));
    let wide = widened(frame(&noise(50_000, 9)), 14, 2);
    for (name, second) in [("narrow", &narrow), ("wide", &wide)] {
        // A skippable frame places the second frame's header across the first buffer's
        // end, from one byte inside it to the whole header.
        for inside in 1..=MAX_HEADER {
            let lead = skippable(input - inside);
            let stream = [&lead[..], second, &narrow].concat();
            let case = format!("{name} header with {inside} bytes in the first buffer");
            let charge = same_as_before(&stream, &case);
            assert_eq!(charge > 0, name == "wide", "{case}");
            for cut in [input - inside + 2, input - 1, input, input + 1, input + 9] {
                same_as_before(&stream[..cut], &format!("{case}, cut at {cut}"));
            }
        }
    }
    // A damaged block after the split fails after the same output as before, which depends
    // on the chunks of input offered: the header's start is a chunk of its own.
    let (narrow, wide, blocks) = raw_block_frames();
    for (name, second) in [("narrow", &narrow), ("wide", &wide)] {
        for inside in 1..=7 {
            let lead = skippable(input - inside);
            let stream = [&lead[..], second].concat();
            for block in [1, 2, blocks.len() - 1] {
                let mut damaged = stream.clone();
                damaged[input - inside + blocks[block]] |= 0b110;
                let case = format!("{name} header with {inside} bytes in, block {block} damaged");
                same_as_before(&damaged, &case);
            }
        }
    }
}

/// What one scripted read does.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Fills the buffer, or reads what is left.
    Full,
    /// Reads at most this many bytes.
    Short(usize),
    /// Fails with `Interrupted`, reading nothing, which a caller retries.
    Interrupted,
    /// Fails with another error, reading nothing.
    Fail,
    /// Reads nothing, before the end of the input, as a growing file does.
    Nothing,
}

/// A reader over `bytes` whose reads follow `script`, by the index of each read.
struct Scripted<'a> {
    bytes: &'a [u8],
    position: usize,
    reads: std::rc::Rc<std::cell::Cell<usize>>,
    script: &'a dyn Fn(usize) -> Step,
}

impl io::Read for Scripted<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let index = self.reads.get();
        self.reads.set(index + 1);
        let rest = &self.bytes[self.position..];
        let length = match (self.script)(index) {
            Step::Interrupted => return Err(io::Error::from(io::ErrorKind::Interrupted)),
            Step::Fail => return Err(io::Error::other("scripted failure")),
            Step::Nothing => 0,
            Step::Short(most) => most.min(buf.len()).min(rest.len()),
            Step::Full => buf.len().min(rest.len()),
        };
        buf[..length].copy_from_slice(&rest[..length]);
        self.position += length;
        Ok(length)
    }
}

/// What a decoder yields from `bytes` read by `script`, and how many reads it made.
fn scripted(bytes: &[u8], script: &dyn Fn(usize) -> Step, frame_loop: bool) -> (Outcome, usize) {
    let reads = std::rc::Rc::new(std::cell::Cell::new(0));
    let stored = Scripted { bytes, position: 0, reads: std::rc::Rc::clone(&reads), script };
    let admission = MemoryAdmission::unlimited();
    let outcome = if frame_loop {
        drain(&mut *decode(stored, Representation::Zstd, &admission).unwrap())
    } else {
        let decoder = zstd::stream::read::Decoder::new(stored).unwrap();
        drain(&mut BufReader::with_capacity(READ_BUFFER_BYTES, decoder))
    };
    (outcome, reads.get())
}

/// Asserts that the frame loop decodes `bytes` read by `script` as zstd's reader does,
/// with the same reads; returns how many reads that took.
fn same_reads_as_before(bytes: &[u8], script: &dyn Fn(usize) -> Step, case: &str) -> usize {
    let ((ours, our_error), our_reads) = scripted(bytes, script, true);
    let ((theirs, their_error), their_reads) = scripted(bytes, script, false);
    assert_eq!(our_error, their_error, "{case}: the error");
    assert!(ours == theirs, "{case}: {} bytes against {}", ours.len(), theirs.len());
    assert_eq!(our_reads, their_reads, "{case}: the reads");
    our_reads
}

/// Streams whose reads include refills inside a frame and a look-ahead for a header split
/// across the input buffer, with narrow and wide frames.
fn scripted_streams() -> Vec<(String, Vec<u8>)> {
    let input = usize::try_from(ZSTD_INPUT_BUFFER).unwrap();
    let narrow = frame(&noise(60_000, 7));
    let wide = widened(frame(&noise(50_000, 9)), 14, 2);
    let (raw, raw_wide, _) = raw_block_frames();
    let mut streams =
        vec![("raw blocks".to_owned(), raw), ("raw blocks, wide".to_owned(), raw_wide)];
    for (name, second) in [("narrow", &narrow), ("wide", &wide)] {
        for inside in [1, 3, 5, 8] {
            let stream = [&skippable(input - inside)[..], second, &narrow].concat();
            streams
                .push((format!("{name} header with {inside} bytes in the first buffer"), stream));
        }
    }
    streams
}

#[test]
fn a_failed_or_interrupted_read_leaves_the_loop_where_it_was() {
    for (name, stream) in scripted_streams() {
        let reads = same_reads_as_before(&stream, &|_| Step::Full, &name);
        assert!(reads > 2, "{name}: {reads} reads");
        // Each read in turn fails once: a retried `Interrupted` read resumes exactly, and
        // any other failure ends the stream after the same output, with the same error.
        for failing in 0..=reads {
            for step in [Step::Interrupted, Step::Fail] {
                let script = move |index| if index == failing { step } else { Step::Full };
                same_reads_as_before(
                    &stream,
                    &script,
                    &format!("{name}, {step:?} at read {failing}"),
                );
            }
        }
        // Every read interrupted once before it succeeds.
        let every = |index: usize| if index % 2 == 0 { Step::Interrupted } else { Step::Full };
        same_reads_as_before(&stream, &every, &format!("{name}, every read interrupted once"));
    }
}

#[test]
fn short_reads_decode_as_before_when_they_complete_each_header() {
    for (name, stream) in scripted_streams() {
        for most in [18, 1_000, 65_536, 131_074] {
            let script = move |_| Step::Short(most);
            same_reads_as_before(&stream, &script, &format!("{name}, reads of {most} bytes"));
        }
    }
    // Reads too short to complete a header only route a narrow frame, which the slot's
    // decoder takes either way.
    let narrow = [frame(&records(400)), frame(&records(300))].concat();
    for most in [1, 2, 5, 7] {
        let script = move |_| Step::Short(most);
        same_reads_as_before(&narrow, &script, &format!("narrow frames, reads of {most} bytes"));
    }
}

#[test]
fn a_look_ahead_that_finds_the_end_of_the_input_ends_it() {
    // The first read ends 3 bytes into the second frame's header; the look-ahead read
    // finds nothing, as at the end of a file a compressor is still writing, and the rest
    // arrives only after it.
    let first = frame(&records(100));
    let split = first.len() + 3;
    let stream = [first, frame(&records(200))].concat();
    for second in [Step::Nothing, Step::Interrupted] {
        let script = move |index| match index {
            0 => Step::Short(split),
            1 => second,
            _ => Step::Full,
        };
        same_reads_as_before(&stream, &script, &format!("look-ahead {second:?}"));
    }
    let script = |index| match index {
        0 => Step::Short(split),
        1 => Step::Nothing,
        _ => Step::Full,
    };
    let ((decoded, error), reads) = scripted(&stream, &script, true);
    assert_eq!(decoded, records(100));
    assert_eq!(error, Some((io::ErrorKind::UnexpectedEof, "incomplete frame".to_owned())));
    assert_eq!(reads, 2, "nothing is read after the end of the input");
}

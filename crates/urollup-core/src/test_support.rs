//! Helpers shared by unit tests across modules.

/// A deterministic Fisher-Yates shuffle driven by a splitmix64 stream, so a property test's
/// permutation is reproducible from its seed.
pub(crate) fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed;
    for index in (1..items.len()).rev() {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        let bound = u64::try_from(index).unwrap() + 1;
        let pick = usize::try_from(z % bound).unwrap();
        items.swap(index, pick);
    }
}

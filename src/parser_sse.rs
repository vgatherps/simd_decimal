use std::arch::x86_64::{
    _mm_andnot_si128, _mm_cmpeq_epi8, _mm_cvtsi128_si64, _mm_madd_epi16, _mm_maddubs_epi16,
    _mm_max_epu8, _mm_movemask_epi8, _mm_packs_epi32, _mm_set1_epi8, _mm_setr_epi16, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_sub_epi8, _mm_test_all_ones,
};

use crate::tables::{DOT_SHUFFLE_CONTROL, EXPONENT_FROM_BITS, LENGTH_SHIFT_CONTROL};
use crate::{ParseInput, ParseOutput};

/// Parses the inputs passed into (mantissa, exponent) pairs.
/// If any of them detected invalid, returns false
/// # Safety
///
/// It is unsafe to pass anything with a real_length that is greater than 16
#[inline]
pub unsafe fn do_parse_decimals<const N: usize, const KNOWN_INTEGER: bool>(
    inputs: &[ParseInput; N],
    outputs: &mut [ParseOutput; N],
) -> bool {
    let ascii = _mm_set1_epi8(b'0' as i8);
    let dot = _mm_set1_epi8((b'.').wrapping_sub(b'0') as i8);
    let mut cleaned = [_mm_set1_epi8(0); N];

    // PERF
    // I did some expermients to hoist the dot-discovery code above the length shifting code,
    // to try and remove a data dependency. This surprisingly really hurt performance,
    // although in theory it should be a significant improvement as you remove a data dependency
    // from the shift to the dot discovery...

    // This is done as a series of many loops to maximise the instant parallelism available to the
    // cpu. It's semantically identical but means the decoder doesn't have to churn through
    // many copies of the code to find independent instructions

    // first, load data and subtract off the ascii mask
    // Everything in the range '0'..'9' will become 0..9
    // everthing else will overflow into 10..256
    for i in 0..N {
        // transumte will just compile to the intrinsics anyways
        let loaded = std::mem::transmute(*inputs[i].data);
        cleaned[i] = _mm_sub_epi8(loaded, ascii);
    }

    // now, we convert the string from [1234.123 <garbage>] into [00000 ... 1234.123]
    // as well as insert zeros for everything past the end

    // For known-short strings, replacing this with a shift might reduce
    // contention on port 5 (the shuffle port). You can't do this for a full vector
    // since there's no way to do so without an immediate value
    for i in 0..N {
        let shift_mask = LENGTH_SHIFT_CONTROL
            .vecs
            .get_unchecked(inputs[i].real_length);

        cleaned[i] = _mm_shuffle_epi8(cleaned[i], *shift_mask);
    }

    if !KNOWN_INTEGER {
        for i in 0..N {
            let is_eq_dot = _mm_cmpeq_epi8(cleaned[i], dot);
            // Set the top 16 bits to 1 as an implicit dot
            let is_dot_mask = _mm_movemask_epi8(is_eq_dot) as u32 | 0xffff_0000;

            let dot_idx = is_dot_mask.trailing_zeros();

            outputs[i].exponent = EXPONENT_FROM_BITS[dot_idx as usize];
            let dot_control = DOT_SHUFFLE_CONTROL.vecs.get_unchecked(dot_idx as usize);

            cleaned[i] = _mm_shuffle_epi8(cleaned[i], *dot_control);
        }
    }

    let mut all_masks = _mm_set1_epi8(-1);
    for cl in &cleaned {
        // take the unsigned max of '9' and anything in the vector
        // then check for equality to '9'

        let nine = _mm_set1_epi8(9);

        let max_of_nine = _mm_max_epu8(nine, *cl);

        // Sub can run on more ports than equality comparison
        let remaining = _mm_sub_epi8(nine, max_of_nine);

        all_masks = _mm_andnot_si128(remaining, all_masks);
    }

    let any_bad_ones = _mm_test_all_ones(all_masks);

    // Now, all that we do is convert to an actual integer

    // Take pairs of u8s (digits) and multiply the more significant one by 10,
    // and accumulate into pairwise u16
    for cl in &mut cleaned {
        let mul_1_10 = _mm_setr_epi8(10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1);
        *cl = _mm_maddubs_epi16(*cl, mul_1_10);
    }

    // Take pairs of u16s (not digits, but two digits each)
    // multiply the more significant by 100 and add to get pairwise u32
    for cl in &mut cleaned {
        let mul_1_100 = _mm_setr_epi16(100, 1, 100, 1, 100, 1, 100, 1);
        *cl = _mm_madd_epi16(*cl, mul_1_100);
    }

    // We now have pairwise u32s, but there are no methods to multiply and horizontally add
    // them. Doing it outright is *very* slow.
    // We know that nothing yet can be larger than 2^16, so we pack the u16s
    // into the first and second half of the vector
    // Each vector half will now be identical.

    for cl in &mut cleaned {
        *cl = _mm_packs_epi32(*cl, *cl);
    }

    // Two choices with similar theoretical performance, afaik.
    // One is that we do one more round of multiply-accumulate in simd, then exit to integer
    // The other is that we do some swar games on what we've just packed into the first 64 bytes.
    // The simd one *I think* faster. Higher throughput, less instructions to issue
    // but might compete with the other madd slots a but more
    // The swar one:
    // 1. is more complex
    // 2. *might* compete with some of the exponent code for integer slot
    // 3. mul is potentially lower throughput than madd
    // 4. Doesn't require load slots for the constant (low impact imo)
    // will just have to benchmark both

    for cl in &mut cleaned {
        let mul_1_10000 = _mm_setr_epi16(10000, 1, 10000, 1, 10000, 1, 10000, 1);
        *cl = _mm_madd_epi16(*cl, mul_1_10000);
    }

    let mut u32_pairs = [0; N];
    for i in 0..N {
        u32_pairs[i] = _mm_cvtsi128_si64(cleaned[i]) as u64;
    }

    for i in 0..N {
        let small_bottom = u32_pairs[i] >> 32;

        // I used to have some code here where you could statically specify
        // there were less than 8 digits, but it had almost no performance impact

        let large_half = u32_pairs[i] as u32 as u64;
        outputs[i].mantissa = 100000000 * large_half + small_bottom;
    }

    any_bad_ones == 1
}

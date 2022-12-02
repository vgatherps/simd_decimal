use std::arch::x86_64::{
    _mm_andnot_si128, _mm_cmpeq_epi8, _mm_cvtsi128_si64, _mm_madd_epi16, _mm_maddubs_epi16,
    _mm_max_epu8, _mm_movemask_epi8, _mm_packs_epi32, _mm_set1_epi8, _mm_setr_epi16, _mm_setr_epi8,
    _mm_shuffle_epi8, _mm_sub_epi8, _mm_test_all_ones, _mm_tzcnt_32,
};

use crate::tables::{DOT_SHUFFLE_CONTROL, EXPONENT_FROM_BITS, LENGTH_SHIFT_CONTROL};

#[derive(Clone, Copy, Debug)]
pub struct ParseInput<'a> {
    pub data: &'a [u8; 16],
    pub real_length: usize,
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct ParseOutput {
    pub mantissa: u64,
    pub exponent: u8,
}

// While plain sse gets most of the way there,
// THis tends to have better perf with full 'native' instruction set
// haven't bothered to go through asm and see what the difference is
// probably some intrinsic getting outlined
// It only seems to impact the integers benchmarks though

/// Parses the inputs passed into (mantissa, exponent) pairs.
/// If any of them detected invalid, returns false
/// # Safety
///
/// It is unsafe to pass anything with a real_length that is greater than 16
#[target_feature(enable = "sse4.2,bmi1,bmi2")]
pub unsafe fn do_parse_many_decimals<const N: usize, const KNOWN_INTEGER: bool>(
    inputs: &[ParseInput; N],
    outputs: &mut [ParseOutput; N],
) -> bool {
    let ascii = _mm_set1_epi8(b'0' as i8);
    let dot = _mm_set1_epi8((b'.').wrapping_sub(b'0') as i8);
    let mut cleaned = [_mm_set1_epi8(0); N];
    let mut dot_idx = [0; N];

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

            let local_dot_idx = _mm_tzcnt_32(is_dot_mask) as u32;

            outputs[i].exponent = EXPONENT_FROM_BITS[local_dot_idx as usize];
            let dot_control = DOT_SHUFFLE_CONTROL
                .vecs
                .get_unchecked(local_dot_idx as usize);

            cleaned[i] = _mm_shuffle_epi8(cleaned[i], *dot_control);
            dot_idx[i] = local_dot_idx;
        }
    }

    let mut all_masks = _mm_set1_epi8(-1);
    // mix validation and exponent calculation as these are fully independent already
    // and don't overlap live register sets
    for i in 0..N {
        // take the unsigned max of '9' and anything in the vector
        // then check for equality to '9'

        let nine = _mm_set1_epi8(9);

        let max_of_nine = _mm_max_epu8(nine, cleaned[i]);

        // Sub can run on more ports than equality comparison
        let remaining = _mm_sub_epi8(nine, max_of_nine);

        all_masks = _mm_andnot_si128(remaining, all_masks);
    }

    let any_bad_ones = _mm_test_all_ones(all_masks);

    if any_bad_ones != 1 {
        return false;
    }

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

    true
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_a_big_decimal() {
        let data = b"987654321.123_..";
        let real_length = 13;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_many_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 3,
                mantissa: 987654321123
            }
        );
    }

    #[test]
    fn test_a_big_integer() {
        let data = b"987654321123_..9";
        let real_length = 12;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_many_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 0,
                mantissa: 987654321123
            }
        );
    }

    #[test]
    fn test_full_sized_integer() {
        let data = b"1234567898765432";
        let real_length = 16;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_many_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 0,
                mantissa: 1234567898765432
            }
        );
    }

    #[test]
    fn test_max_integer() {
        let data = b"9999999999999999";
        let real_length = 16;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_many_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 0,
                mantissa: 9999999999999999
            }
        );
    }

    #[test]
    fn test_min_decimal() {
        let data = b".000000000000001";
        let real_length = 16;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_many_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 15,
                mantissa: 1
            }
        );
    }
}

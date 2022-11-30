pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

/*


// TODO store side by side?
// if we make each one take up the same space, we save an instruction?
extern __m128i dot_shuffle_control[16];
extern __m128i length_shift_control[16];

__attribute__((cold, noreturn, noinline)) void bail_on_bad_integer();

struct CondensedDecimal {
  // The original string, with the decimal point removed,
  // shifted so that it ends on the final byte and is prefixed with zeros
  //
  // so 1234.13432 followed by garbage would become
  // 0000000123413432
  // dot at original index 4
  // original length 10
  std::int64_t int_value;
  std::uint32_t dot;
  std::uint32_t length;
};

#define arr(T) std::array<T, N>
#define FOR(i) for (int i = 0; i < N; i++)
#define SPLIT_FOR(i)                                                           \
  }                                                                            \
  FOR(i) {

// TODO do this in terms of pairs of strings

// Takes a series of 15-digit decimal strings aaaa.bbbbb, and converts them into
// 15 digit integers with length and dot position recorded.
//
// You lose performance by *not* doing this in stages
// so it's somewhat hard to make nice clean segregated tests
template <std::size_t N>
void __attribute__((noinline))
condense_string_arr(const __m128i *data, CondensedDecimal *out, bool &is_bad) {
  // have to mask off via length to avoid spuriosuly finding new dots
  __m128i find_dot = _mm_set1_epi8('.');
  arr(__m128i) cleaned;
  std::uint32_t sans_first_dot = 0;
  // Schedule loads and comparisons of data
  FOR(i) {
    // Set the last character to zero. This is used by the shuffles to
    // forward fill leading zeros
    cleaned[i] = _mm_insert_epi8(data[i], '0', 15);
    SPLIT_FOR(i)
    // Project the string down to the far end - this gives us many
    // leading zeros, instead of many trailing zeros
    // The reason that we do this, is because we then can parse directly
    // into the natural scale.
    // Alternatively, we can *always* parse into a 15-digit scale,
    // however we then may potentially have to divide the result back due to the
    // dot placement

    // A potentially faster way to do this is
    // 1. When loading, prepare a 32-byte space of '0'
    // 2. Get the length of the string
    // 3. Store (unaligned) so that the number ends at the tail
    //    garbage in the later 16 bytes and '000000<number>' in the head bytes
    // The above might be slow due to conflicts in the store buffer and
    // alignment issues?
    cleaned[i] =
        _mm_shuffle_epi8(cleaned[i], length_shift_control[out[i].length]);

    SPLIT_FOR(i)
    // Locate the dot. This is after shifting to the other end of the array
    // This is also done in the ascii-adjusted space
    __m128i is_eq_dot = _mm_cmpeq_epi8(find_dot, cleaned[i]);
    std::uint32_t is_dot = _mm_movemask_epi8(is_eq_dot);
    // aggregate all of the validation masks - discover if there are any bad
    // nonzero masks
    is_bad |= (is_dot & (is_dot - 1)) != 0;
    // schedule discover of the dot index.
    std::uint32_t dot_idx = __tzcnt_u32(is_dot);
    out[i].dot = dot_idx;
    // Shift the dot outside of the array, if it exists
    cleaned[i] = _mm_shuffle_epi8(cleaned[i], dot_shuffle_control[dot_idx]);

    // Now for the conversion into numbers
    SPLIT_FOR(i)
    const __m128i ascii0 = _mm_set1_epi8('0');

    // 1. convert from ASCII '0' .. '9' to numbers 0 .. 9
    cleaned[i] = _mm_sub_epi8(cleaned[i], ascii0);
    SPLIT_FOR(i)
    // 2. convert to 2-digit numbers
    const __m128i mul_1_10 =
        _mm_setr_epi8(10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1);
    cleaned[i] = _mm_maddubs_epi16(cleaned[i], mul_1_10);
    SPLIT_FOR(i)
    // 3. convert to 4-digit numbers
    const __m128i mul_1_100 = _mm_setr_epi16(100, 1, 100, 1, 100, 1, 100, 1);
    cleaned[i] = _mm_madd_epi16(cleaned[i], mul_1_100);
    SPLIT_FOR(i)
    // 4a. convert from 32-bit into 16-bit element vector
    //   We know that nothing can exceed 16 bytes, and there is no 32 bit
    //   version of _mm_madd_epi16
    //   This basically duplicates our value to the front and back
    //   However I think that going the unpacklo route might be better?
    //   This instruction appears to get slower on newer intel cpus,
    //   a byte shuffle would 100% work
    //   HOWEVER this would require loading an extra state register,
    //   likely free given the whole pipeline?
    cleaned[i] = _mm_packus_epi32(cleaned[i], cleaned[i]);
    SPLIT_FOR(i)
    // 4. convert to 8-digit numbers
    const __m128i mul_1_10000 =
        _mm_setr_epi16(10000, 1, 10000, 1, 10000, 1, 10000, 1);
    cleaned[i] = _mm_madd_epi16(cleaned[i], mul_1_10000);
    SPLIT_FOR(i)
    out[i].int_value =
        ((std::uint64_t)(std::uint32_t)_mm_cvtsi128_si32(cleaned[i])) *
            100000000 +
        (std::uint64_t)(std::uint32_t)_mm_extract_epi32(cleaned[i], 1);
  }
}
*/

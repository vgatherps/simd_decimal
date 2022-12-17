//! This crate provides vectorized decimal parsing functions for x86 and aarch64
//! There is exactly one interface -

#[cfg(target_arch = "x86_64")]
mod parser_sse;
#[cfg(target_arch = "x86_64")]
use parser_sse::parse_decimals;

#[cfg(target_arch = "aarch64")]
mod parser_aarch64;
#[cfg(target_arch = "aarch64")]
use parser_aarch64::do_parse_decimals;

mod tables;

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
#[inline]
/// Parses the inputs passed into (mantissa, exponent) pairs.
/// If any of them detected invalid, returns false
///
/// No doctests for this dummy wrapper since they'll fail on unsupported architectures
///
/// # Safety
///
/// It is unsafe to pass anything with a real_length that is greater than 16
pub unsafe fn parse_decimals<const N: usize, const I: bool>(
    _: &[ParseInput; N],
    _: &mut [ParseOutput; N],
) -> bool {
    false
}

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
#[inline]
/// Parses the inputs passed into (mantissa, exponent) pairs, and returns false if one is detected to be invalid
///
/// # Safety
///
/// It is unsafe to pass anything with a real_length that is greater than 16
///
/// Examples:
///
/// ```
/// let data = b"987654321.123_..";
/// let real_length = 13;
/// let input = ParseInput { data, real_length };
/// let mut output = [ParseOutput::default()];
///
/// let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };
///
/// assert!(was_good);
/// assert_eq!(
///     output[0],
///     ParseOutput {
///         exponent: 3,
///         mantissa: 987654321123
///     }
/// );
/// ```
///
pub unsafe fn parse_decimals<const N: usize, const KNOWN_INTEGER: bool>(
    inputs: &[ParseInput; N],
    outputs: &mut [ParseOutput; N],
) -> bool {
    do_parse_decimals::<N, KNOWN_INTEGER>(inputs, outputs)
}

/// Struct containing descriptors of the input to be parsed.
/// Specifically this contains a reference to 16 contiguous characters starting with the
/// decimal itself that are valid to load, as well as the true number length
#[derive(Clone, Copy, Debug)]
pub struct ParseInput<'a> {
    /// Reference to 16 contiguous bytes with the number starting from the least significant bytes
    /// i.e. if the whole string was "12345.234, random junk bytes"
    /// this would refer to "12345.234, rando"
    pub data: &'a [u8; 16],

    /// This is the actual length of the decimal. In the above example,
    /// with the 16 bytes being "12345.234, rando",
    /// the real length is 9. &"12345.234, rando"[..9] = "12345.234"
    pub real_length: usize,
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct ParseOutput {
    pub mantissa: u64,
    pub exponent: u8,
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_zero() {
        let data = [b'0'; 16];
        for real_length in 1..16 {
            let input = ParseInput {
                data: &data,
                real_length,
            };
            let mut output = [ParseOutput::default()];

            let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

            assert!(was_good);
            assert_eq!(
                output[0],
                ParseOutput {
                    exponent: 0,
                    mantissa: 0
                }
            );
        }
    }

    #[test]
    fn test_a_big_decimal() {
        let data = b"987654321.123_..";
        let real_length = 13;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 15,
                mantissa: 1
            }
        );
    }

    #[test]
    fn test_dot_at_end() {
        let data = b"987654321.------";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 0,
                mantissa: 987654321
            }
        );
    }

    #[test]
    fn test_dot_at_start() {
        let data = b".987654321------";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(was_good);
        assert_eq!(
            output[0],
            ParseOutput {
                exponent: 9,
                mantissa: 987654321
            }
        );
    }

    #[test]
    fn test_multiple_dots() {
        let data = b"..987654321-----";
        let real_length = 4;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }

    #[test]
    fn test_invalid_separator() {
        let data = b".9876_54321-----";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }

    #[test]
    #[allow(clippy::octal_escapes)]
    fn test_zero_inside() {
        let data = b".9876\054321-----";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { do_parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }
}

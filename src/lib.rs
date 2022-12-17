#[cfg(target_arch = "x86_64")]
mod parser_sse;
#[cfg(target_arch = "x86_64")]
pub use parser_sse::parse_decimals;

#[cfg(target_arch = "aarch64")]
mod parser_aarch64;
#[cfg(target_arch = "aarch64")]
pub use parser_aarch64::parse_decimals;

mod tables;

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
#[inline]
pub unsafe fn parse_decimals<const N: usize, const I: bool>(
    _: &[ParseInput; N],
    _: &mut [ParseOutput; N],
) -> bool {
    false
}

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

            let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

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

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }

    #[test]
    fn test_invalid_separator() {
        let data = b".9876_54321-----";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }

    #[test]
    #[allow(clippy::octal_escapes)]
    fn test_zero_inside() {
        let data = b".9876\054321-----";
        let real_length = 10;
        let input = ParseInput { data, real_length };
        let mut output = [ParseOutput::default()];

        let was_good = unsafe { parse_decimals::<1, false>(&[input], &mut output) };

        assert!(!was_good);
    }
}

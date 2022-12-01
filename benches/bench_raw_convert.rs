use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::{parse_str_radix_10, DecConverter};
use simd_decimal::*;

// No point in having different integers since the algorithm does not branch
// or behave differently based on size
const BASE: &[u8; 16] = b"9876543211231_..";
const BASE_STR: &str = "92323423";

const BASE_INPUT: ParseInput = ParseInput {
    data: BASE,
    real_length: 13,
};

// Put us and rust_decimal on the same playing field
pub struct RawParsedDec {
    pub value: u128,
    pub scale: u8,
}

impl DecConverter for RawParsedDec {
    type Output = RawParsedDec;
    type Error = &'static str;

    fn convert<const NEG: bool>(mantissa: u128, scale: u8) -> Result<Self::Output, Self::Error> {
        Ok(Self {
            value: mantissa,
            scale,
        })
    }

    #[inline]
    fn tail_error(err: &'static str) -> Result<Self::Output, Self::Error> {
        Err(err)
    }
}

const MANY: [ParseInput; 16] = [BASE_INPUT; 16];

fn run_bench_for<const N: usize, const INT: bool>(c: &mut Criterion) {
    let real_input: &[ParseInput; N] = (&MANY[..N]).try_into().unwrap();
    let mut outputs = [ParseOutput::default(); N];

    c.bench_function(
        &format!("Raw parse batch of {} int {}", N, INT),
        |b| unsafe {
            let fnc = || {
                let rval = do_parse_many_decimals::<N, INT>(
                    black_box(real_input),
                    black_box(&mut outputs),
                );
                black_box(&outputs);
                black_box(rval);
            };

            b.iter(fnc);
        },
    );
}

fn run_decimal_bench_for<const N: usize>(c: &mut Criterion) {
    c.bench_function(&format!("Decimal parse batch of {}", N), |b| {
        let fnc = || {
            for _ in 0..N {
                black_box(parse_str_radix_10::<RawParsedDec>(BASE_STR).unwrap());
            }
        };

        b.iter(fnc);
    });
}

fn run_float_bench_1(c: &mut Criterion) {
    run_bench_for::<1, false>(c);
}

fn run_float_bench_2(c: &mut Criterion) {
    run_bench_for::<2, false>(c);
}

fn run_float_bench_4(c: &mut Criterion) {
    run_bench_for::<4, false>(c);
}

fn run_float_bench_8(c: &mut Criterion) {
    run_bench_for::<8, false>(c);
}

fn run_float_bench_16(c: &mut Criterion) {
    run_bench_for::<16, false>(c);
}

fn run_dec_bench_1(c: &mut Criterion) {
    run_decimal_bench_for::<1>(c);
}

fn run_dec_bench_2(c: &mut Criterion) {
    run_decimal_bench_for::<2>(c);
}

fn run_dec_bench_4(c: &mut Criterion) {
    run_decimal_bench_for::<4>(c);
}

fn run_dec_bench_8(c: &mut Criterion) {
    run_decimal_bench_for::<8>(c);
}

fn run_dec_bench_16(c: &mut Criterion) {
    run_decimal_bench_for::<16>(c);
}

fn run_integer_bench_1(c: &mut Criterion) {
    run_bench_for::<1, true>(c);
}

fn run_integer_bench_2(c: &mut Criterion) {
    run_bench_for::<2, true>(c);
}

fn run_integer_bench_4(c: &mut Criterion) {
    run_bench_for::<4, true>(c);
}

fn run_integer_bench_8(c: &mut Criterion) {
    run_bench_for::<8, true>(c);
}

fn run_integer_bench_16(c: &mut Criterion) {
    run_bench_for::<16, true>(c);
}

criterion_group!(
    raw_parse_benches,
    run_float_bench_1,
    run_integer_bench_1,
    run_float_bench_1,
    run_integer_bench_1,
    run_float_bench_2,
    run_integer_bench_2,
    run_float_bench_4,
    run_integer_bench_4,
    run_float_bench_8,
    run_integer_bench_8,
    run_float_bench_16,
    run_integer_bench_16,
);

criterion_group!(
    decimal_parse_benches,
    run_dec_bench_1,
    run_dec_bench_2,
    run_dec_bench_4,
    run_dec_bench_8,
    run_dec_bench_16,
);
criterion_main!(raw_parse_benches, decimal_parse_benches);

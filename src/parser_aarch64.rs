use std::arch::aarch64::{
    vaddvq_u64, vceqq_u8, vcgeq_u8, vdupq_n_u8, vget_lane_u64, vget_low_u16, vget_low_u32,
    vget_low_u8, vgetq_lane_u64, vmlal_high_n_u16, vmlal_high_n_u32, vmlal_high_u8, vmovl_u16,
    vmovl_u32, vmovl_u8, vorrq_u8, vqtbl1q_u8, vreinterpret_u64_u8, vreinterpretq_u16_u8,
    vreinterpretq_u32_u8, vreinterpretq_u64_u8, vreinterpretq_u8_u16, vreinterpretq_u8_u32,
    vreinterpretq_u8_u64, vshrn_n_u16, vsubq_u8,
};

use crate::tables::{VecCharArray, DOT_SHUFFLE_CONTROL, EXPONENT_FROM_BITS, LENGTH_SHIFT_CONTROL};
use crate::{ParseInput, ParseOutput};

// base_1 conversion back and forth
const fn a(idx: u8) -> u8 {
    let base_zero = idx - 1;
    base_zero * 2
}

const fn b(idx: u8) -> u8 {
    let base_zero = idx - 1;
    1 + base_zero * 2
}

// This is specified in reverse of the derivation because flipping the shuffle order
// is faster
const SHUFFLE_ACC: VecCharArray<1> = VecCharArray {
    chars: [[
        b(8),
        b(4),
        b(6),
        b(2),
        b(7),
        b(3),
        b(5),
        b(1),
        a(8),
        a(4),
        a(6),
        a(2),
        a(7),
        a(3),
        a(5),
        a(1),
    ]],
};

// aarch64 version of the sse parser. Most documentation is there.

/// Parses the inputs passed into (mantissa, exponent) pairs.
/// If any of them detected invalid, returns false
/// # Safety
///
/// It is unsafe to pass anything with a real_length that is greater than 16
pub unsafe fn parse_decimals<const N: usize, const KNOWN_INTEGER: bool>(
    inputs: &[ParseInput; N],
    outputs: &mut [ParseOutput; N],
) -> bool {
    let ascii = vdupq_n_u8(b'0');
    let dot = vdupq_n_u8((b'.').wrapping_sub(b'0'));
    let mut cleaned = [vdupq_n_u8(0); N];

    for i in 0..N {
        // transumte will just compile to the intrinsics anyways
        let loaded = std::mem::transmute(*inputs[i].data);
        cleaned[i] = vsubq_u8(loaded, ascii);
    }

    for i in 0..N {
        let shift_mask = LENGTH_SHIFT_CONTROL
            .vecs
            .get_unchecked(inputs[i].real_length);

        cleaned[i] = vqtbl1q_u8(cleaned[i], *shift_mask);
    }

    // https://community.arm.com/arm-community-blogs/b/infrastructure-solutions-blog/posts/porting-x86-vector-bitmask-optimizations-to-arm-neon
    if !KNOWN_INTEGER {
        let mut exploded_dot_mask: [u64; N] = [0; N];
        let mut dot_idx: [u32; N] = [0; N];
        for i in 0..N {
            let is_eq_dot = vceqq_u8(cleaned[i], dot);

            let is_eq_fot_16 = vreinterpretq_u16_u8(is_eq_dot);

            let is_eq_dot_4x_vec_mask = vshrn_n_u16(is_eq_fot_16, 4);

            exploded_dot_mask[i] = vget_lane_u64(vreinterpret_u64_u8(is_eq_dot_4x_vec_mask), 0);
        }

        for i in 0..N {
            dot_idx[i] = exploded_dot_mask[i].trailing_zeros() / 4;
        }

        for i in 0..N {
            // arm has a fast saturating sub instruction

            outputs[i].exponent = *EXPONENT_FROM_BITS.get_unchecked(dot_idx[i] as usize);

            let dot_control = DOT_SHUFFLE_CONTROL.vecs.get_unchecked(dot_idx[i] as usize);

            cleaned[i] = vqtbl1q_u8(cleaned[i], *dot_control);
        }
    }

    let mut all_masks = vdupq_n_u8(0);

    let ten = vdupq_n_u8(10);
    for cl in &cleaned {
        let greater_equal_ten = vcgeq_u8(*cl, ten);

        all_masks = vorrq_u8(all_masks, greater_equal_ten);
    }

    // arm version to test all zeros
    let any_bad_ones = vaddvq_u64(vreinterpretq_u64_u8(all_masks));

    if any_bad_ones != 0 {
        return false;
    }

    // Now, all that we do is convert to an actual integer

    // This is done totally differently for arm,
    // as the add-accumulate is different

    // A note to the below -
    // vmlal preserves shufflings that don't cross the high-low barrier
    // so I look for an initial shuffle that will correctly reproduce
    // the final desired shuffle

    // Before any shuffle:

    // [a1, b1, a2, b2, a3, b3, a4, b4, a5, b5, a6, b6, a7, b7, a8, b8]
    // [a1, 2, 3, 4, 5, 6, 7, 8, b1, 2, 3, 4, 5, 6, 7, 8]
    // split and vmlal together to get
    // [ab1, ab2, ab3, ab4, ab5, ab6, ab7, ab8], abx = 10*ax + bx

    // then for the 100 it's (with not c and )
    // [ab1c1, ab2d1, ab3c2, ab4d2, ab5c3, ab6d3, ab7dc4, ab8d4]

    // and reshuffle into
    // [c1, c2, c3, c4, d1, d2, d3, d4], and vmlal etc etc
    // so the true AB order is
    // [ab1c1, ab3c2, ad5c3, ad7c4], ab (2x-1) c (x) : ab (2x) d(x)
    // so if we originally shuffled into
    // [a1, a3, a5, a7, a2, a4, a6, a8, b1, b3, b5, b7, b2, b4, b6, b8]
    // we already end up in the correct order!

    // it seems pretty clear to me that the next reordering is going to be
    // [a1, a5, a3, a7, a2, a6, a4, a8, repreat for b]

    // let's check it out
    // finishing the above we have
    // [ab12cd1, ab23cd2, ab45cd3, ab78cd4]

    // repeat with 10000
    // [ab12cd1e1, ab34cd2f1, ab56cd3e2, ab78cd4f2]
    // shuffle into [e1, e2, f1, f2]
    // expaded
    // [ab12cd1e1, ab56cd3e2, ab34cd2f1, ab78cd4f2]
    // vmal gets the highest and lowest bits of each.
    // no more nontrivial shuffles as there are only two vectors left

    // let's consider what happens if we start with the guesstimated correct shuffle
    // [a1, a5, a3, a7, a2, a6, a4, a8, b1, b5, b3, b7, b2, b6, b4, b8]
    // [ab1, ab5, ab3, ab7, ab2, ab6, ab4, ab8]
    // [ab12, ab56, ab34, ab78]
    // [ab1234, ab5678]

    // The last result is still 32-bit, so it looks like
    // [0, ab1234, 0, a5678]
    // This means we can do the final multiplication/add
    // directly with another accumulate
    // to get [0, ab12345678]

    // tada!

    // To make things more confusing, arm does this
    // with fewer instructions if we swap high and low.
    // So the vector mask is actually reversed to the above
    // but no other details change

    let acc_shuffle = SHUFFLE_ACC.vecs[0];
    for cl in &mut cleaned {
        *cl = vqtbl1q_u8(*cl, acc_shuffle);
    }

    // the 8 bit one is a little different
    // since there's fwer available multiply accululate instructions

    // small siwtched to lower bits and large siwtched to high
    // to save a dup instructions in these accumulates
    // only works for th first once since it naturally
    // accumulates larger elements to the lower indices

    // I'm not a neon expert, would be glad to find a faster way to do th

    for cl in &mut cleaned {
        let small = vmovl_u8(vget_low_u8(*cl));
        let acc = vmlal_high_u8(small, *cl, ten);
        *cl = vreinterpretq_u8_u16(acc);
    }

    for cl in &mut cleaned {
        let as_16 = vreinterpretq_u16_u8(*cl);
        let small = vmovl_u16(vget_low_u16(as_16));
        let acc = vmlal_high_n_u16(small, as_16, 1_00);
        *cl = vreinterpretq_u8_u32(acc);
    }

    for cl in &mut cleaned {
        let as_32 = vreinterpretq_u32_u8(*cl);
        let small = vmovl_u32(vget_low_u32(as_32));
        let acc = vmlal_high_n_u32(small, as_32, 1_00_00);
        *cl = vreinterpretq_u8_u64(acc);
    }

    // NEON has no 64-bit integer multiply, sadly.
    // However, we know that the above each fit into a u32 still
    // TODO I'm assuming these multiplications happen in 64-bit space,
    // and that's why there's no vector equivalent for larger.
    // need to test...

    // TO BENCHMARK: should compare the simple swizzle, extract,
    // and perform in integer space?
    for i in 0..N {
        let as_32 = vreinterpretq_u32_u8(cleaned[i]);
        let small = vmovl_u32(vget_low_u32(as_32));
        let acc = vmlal_high_n_u32(small, as_32, 1_00_00_00_00);
        outputs[i].mantissa = vgetq_lane_u64(acc, 0);
    }

    any_bad_ones == 0
}

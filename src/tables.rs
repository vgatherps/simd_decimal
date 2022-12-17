#[cfg(target_arch = "aarch64")]
mod vec {
    use std::arch::aarch64::uint8x16_t;
    pub type Vec = uint8x16_t;
}

#[cfg(target_arch = "x86_64")]
mod vec {
    use std::arch::x86_64::__m128i;
    pub type Vec = __m128i;
}

pub(crate) union VecCharArray<const N: usize> {
    pub vecs: [vec::Vec; N],
    pub chars: [[u8; 16]; N],
}

// vector ops aren't const, so some union tricks to the rescue
pub(crate) const DOT_SHUFFLE_CONTROL: VecCharArray<17> = VecCharArray {
    chars: generate_dot_shuffle_control(),
};

pub(crate) const LENGTH_SHIFT_CONTROL: VecCharArray<17> = VecCharArray {
    chars: generate_length_shift_control(),
};

pub(crate) const EXPONENT_FROM_BITS: [u8; 17] =
    [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0];

const fn generate_single_dot_field(dot: u8, i: u8) -> u8 {
    if i > dot || dot >= 16 {
        i
    } else if i > 0 {
        i - 1
    } else {
        // the dot is at zero, set value to zero fill
        u8::MAX
    }
}

const fn generate_dot_for(dot: u8) -> [u8; 16] {
    // we're compressing everything towards the end
    // 'after' the dot, in raw index order, we do nothing
    // before / at the dot, we shift over from the index ahead

    // hm todo I don't think this properly handles overly large dot idx
    [
        generate_single_dot_field(dot, 0),
        generate_single_dot_field(dot, 1),
        generate_single_dot_field(dot, 2),
        generate_single_dot_field(dot, 3),
        generate_single_dot_field(dot, 4),
        generate_single_dot_field(dot, 5),
        generate_single_dot_field(dot, 6),
        generate_single_dot_field(dot, 7),
        generate_single_dot_field(dot, 8),
        generate_single_dot_field(dot, 9),
        generate_single_dot_field(dot, 10),
        generate_single_dot_field(dot, 11),
        generate_single_dot_field(dot, 12),
        generate_single_dot_field(dot, 13),
        generate_single_dot_field(dot, 14),
        generate_single_dot_field(dot, 15),
    ]
}

const fn generate_dot_shuffle_control() -> [[u8; 16]; 17] {
    [
        generate_dot_for(0),
        generate_dot_for(1),
        generate_dot_for(2),
        generate_dot_for(3),
        generate_dot_for(4),
        generate_dot_for(5),
        generate_dot_for(6),
        generate_dot_for(7),
        generate_dot_for(8),
        generate_dot_for(9),
        generate_dot_for(10),
        generate_dot_for(11),
        generate_dot_for(12),
        generate_dot_for(13),
        generate_dot_for(14),
        generate_dot_for(15),
        generate_dot_for(16),
    ]
}

const fn generate_length_shift_field(length: u8, i: u8) -> u8 {
    // We compress eveything to the very end, while assuming that
    // This lets us shift AND mask all in one go
    // basically, we compress by (16 - length)
    let shift_up_front = 16 - length;
    if i < shift_up_front {
        u8::MAX
    } else {
        i - shift_up_front
    }
}

const fn generate_length_shift_for(length: u8) -> [u8; 16] {
    [
        generate_length_shift_field(length, 0),
        generate_length_shift_field(length, 1),
        generate_length_shift_field(length, 2),
        generate_length_shift_field(length, 3),
        generate_length_shift_field(length, 4),
        generate_length_shift_field(length, 5),
        generate_length_shift_field(length, 6),
        generate_length_shift_field(length, 7),
        generate_length_shift_field(length, 8),
        generate_length_shift_field(length, 9),
        generate_length_shift_field(length, 10),
        generate_length_shift_field(length, 11),
        generate_length_shift_field(length, 12),
        generate_length_shift_field(length, 13),
        generate_length_shift_field(length, 14),
        generate_length_shift_field(length, 15),
    ]
}

const fn generate_length_shift_control() -> [[u8; 16]; 17] {
    [
        generate_length_shift_for(0),
        generate_length_shift_for(1),
        generate_length_shift_for(2),
        generate_length_shift_for(3),
        generate_length_shift_for(4),
        generate_length_shift_for(5),
        generate_length_shift_for(6),
        generate_length_shift_for(7),
        generate_length_shift_for(8),
        generate_length_shift_for(9),
        generate_length_shift_for(10),
        generate_length_shift_for(11),
        generate_length_shift_for(12),
        generate_length_shift_for(13),
        generate_length_shift_for(14),
        generate_length_shift_for(15),
        generate_length_shift_for(16),
    ]
}

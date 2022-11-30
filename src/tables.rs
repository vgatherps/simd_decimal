// Static mut for the test to get around the fact that
// intrinsics expression ARE NOT constexpr, and it's pointless to
// take the compile-time cost of once-cell for something that
// should just be initialized immediately

use std::arch::x86_64::__m128i;

pub(crate) union VecCharArray<const N: usize> {
    pub vecs: [__m128i; N],
    chars: [[u8; 16]; N],
}

// vector ops aren't const, so some union tricks to the rescue
// so it's union tricks to the rescue
pub(crate) const DOT_SHUFFLE_CONTROL: VecCharArray<33> = VecCharArray {
    chars: generate_dot_shuffle_control(),
};

pub(crate) const LENGTH_SHIFT_CONTROL: VecCharArray<17> = VecCharArray {
    chars: generate_length_shift_control(),
};

const fn generate_dot_for(dot: u8) -> [u8; 16] {
    // we're compressing everything towards the end
    // 'after' the dot, in raw index order, we do nothing
    // before / at the dot, we shift over from the index ahead

    // hm todo I don't think this properly handles overly large dot idx
    let mut data = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        if i > dot || dot >= 16 {
            data[i as usize] = i;
        } else if i > 0 {
            data[i as usize] = i - 1;
        } else {
            // the dot is at zero, set value to zero fill
            data[i as usize] = u8::MAX;
        }
        i += 1;
    }
    data
}

const fn generate_dot_shuffle_control() -> [[u8; 16]; 33] {
    let mut data = [[0; 16]; 33];
    let mut i = 0;
    while i <= 32 {
        data[i as usize] = generate_dot_for(i);
        i += 1;
    }
    data
}

const fn generate_length_shift_for(length: u8) -> [u8; 16] {
    // We compress eveything to the very end, while assuming that
    // the last element is zero. This lets us shift AND mask all in one go
    // basically, we compress by (16 - length)
    let mut data = [0; 16];
    let shift_up_front = 16 - length;

    // fill up the front part of the array to select from known zeros
    let mut i = 0;
    while i < shift_up_front {
        // This sets the high bits, making the shuffle fill with zero
        data[i as usize] = u8::MAX;
        i += 1;
    }

    // Fill the later parts of the array to select from the front

    let mut i = 0;
    while i < length {
        data[(i + shift_up_front) as usize] = i;
        i += 1;
    }

    data
}

const fn generate_length_shift_control() -> [[u8; 16]; 17] {
    let mut length_shift_control = [[0; 16]; 17];
    let mut i = 0;
    while i <= 16 {
        length_shift_control[i as usize] = generate_length_shift_for(i);
        i += 1;
    }

    length_shift_control
}

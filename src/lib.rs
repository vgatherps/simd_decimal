#![feature(const_for)]
#![feature(const_mut_refs)]

use std::arch::x86_64::__m128i;

pub mod parser;
mod tables;

pub unsafe fn transmute_char(v: __m128i) -> [u8; 16] {
    std::mem::transmute(v)
}

pub unsafe fn transmute_short(v: __m128i) -> [u16; 8] {
    std::mem::transmute(v)
}

pub unsafe fn transmute_word(v: __m128i) -> [u32; 4] {
    std::mem::transmute(v)
}
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

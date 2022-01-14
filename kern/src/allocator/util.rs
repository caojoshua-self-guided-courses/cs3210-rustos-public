/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
#[allow(dead_code)]
pub fn align_down(addr: usize, align: usize) -> usize {
    verify_power_of_two(align);
    addr - addr % align
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    verify_power_of_two(align);
    if addr % align == 0 {
        addr
    } else {
        addr + align - addr % align
    }
}

/// Returns true if `align` is a power of 2. false otherwise.
///
/// This function uses the fact that a number `x` that is a power of
/// 2 has a single bit, and a number `x - 1` is full of 1 bits to the
/// right of the single set bit in `x`. Therefore, `x & (x - 1)`
/// must be zero. If `y` is not a power of 2, `y` and `y - 1`
/// must share a single bit, and `y & (y - 1)` must not be zero.
pub fn is_power_of_two(align: usize) -> bool {
    align & (align - 1) == 0
}

/// Verifies that `align` is a power of 2. Panics otherwise.r!
///
/// # Panics
fn verify_power_of_two(align: usize) {
    if !is_power_of_two(align) {
        panic!("`{}` is not a power of 2")
    }
}

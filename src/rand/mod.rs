//! Random number generation with hardware entropy.

mod hw;
mod primes;
pub mod urand;

use core::cell::UnsafeCell;
use std::sync::LazyLock;

use primes::PRIMES;

// Re-export urandom control
pub use urand::{enable as enable_urandom, disable as disable_urandom, shutdown as shutdown_urandom};

pub fn is_urandom_enabled() -> bool {
    urand::is_requested()
}

pub fn entropy_source() -> &'static str {
    if urand::is_requested() {
        "/dev/urandom"
    } else {
        hw::source_name()
    }
}

// =============================================================================
// Entropy
// =============================================================================

#[inline(always)]
fn entropy(hint: usize) -> u64 {
    if urand::is_requested() {
        urand::rand(hint)
    } else {
        hw::entropy()
    }
}

// =============================================================================
// RNG
// =============================================================================

static RAND: LazyLock<Rand> = LazyLock::new(Rand::new);

pub struct Rand(UnsafeCell<usize>);
unsafe impl Sync for Rand {}

impl Rand {
    #[inline]
    pub fn new() -> Self {
        Rand(UnsafeCell::new(entropy(0) as usize))
    }

    #[inline(always)]
    pub fn get() -> usize {
        let state = unsafe { *RAND.0.get() };
        let ent = entropy(state) as usize;

        // Mix entropy into prime selection
        let mixed = state ^ ent;
        let idx = (mixed ^ (mixed >> 32)) as usize % PRIMES.len();

        // State transition: rotate, multiply by prime, XOR entropy
        let new_state = state.rotate_left(17).wrapping_mul(PRIMES[idx]) ^ ent;
        unsafe { *RAND.0.get() = new_state };

        // SplitMix64 output finalizer
        let mut z = new_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9_usize);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb_usize);
        z ^ (z >> 31)
    }
}

pub fn zeroize_state() {
    unsafe { std::ptr::write_volatile(RAND.0.get(), 0) }
}

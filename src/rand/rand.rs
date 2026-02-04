use core::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

use super::primes::PRIMES;
use super::urand;

// Runtime flag: use urandom pool instead of hardware entropy
static USE_URANDOM: AtomicBool = AtomicBool::new(false);

/// Check if /dev/urandom is available on this system
pub fn is_urandom_available() -> bool {
    std::path::Path::new("/dev/urandom").exists()
}

/// Enable urandom pool mode (initializes 32MB pool)
/// Returns false if /dev/urandom is not available
pub fn enable_urandom() -> bool {
    if !is_urandom_available() {
        return false;
    }
    urand::init();
    USE_URANDOM.store(true, Ordering::SeqCst);
    true
}

/// Disable urandom pool mode and free memory
pub fn disable_urandom() {
    USE_URANDOM.store(false, Ordering::SeqCst);
    urand::shutdown();
}

/// Check if urandom mode is enabled
pub fn is_urandom_enabled() -> bool {
    USE_URANDOM.load(Ordering::Relaxed)
}

/// Get current entropy source name
pub fn entropy_source() -> &'static str {
    if USE_URANDOM.load(Ordering::Relaxed) {
        "/dev/urandom"
    } else {
        #[cfg(target_arch = "x86_64")]
        { "rdtsc" }
        #[cfg(target_arch = "aarch64")]
        { "cycle counter" }
        #[cfg(target_arch = "arm")]
        { "cycle counter" }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
        { "/dev/urandom" }
    }
}

static RAND: LazyLock<Rand> = LazyLock::new(Rand::new);
pub struct Rand(UnsafeCell<usize>);
unsafe impl Sync for Rand {}

impl Rand {
    #[inline]
    pub fn new() -> Self {
        Rand(UnsafeCell::new(get_entropy() as usize))
    }

    #[inline(always)]
    pub fn get() -> usize {
        let state = unsafe { *RAND.0.get() };
        let entropy = get_entropy() as usize;

        // Mix entropy into prime selection
        let mixed = state ^ entropy;
        let idx = (mixed ^ (mixed >> 32)) as usize % PRIMES.len();

        // State transition: rotate by prime (17), multiply, XOR entropy
        let new_state = state
            .rotate_left(17)
            .wrapping_mul(PRIMES[idx])
            ^ entropy;

        unsafe {
            *RAND.0.get() = new_state;
        }

        // SplitMix64 finalizer for output
        let mut z = new_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9_usize);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb_usize);
        z ^ (z >> 31)
    }
}

// =============================================================================
// Entropy sources
// =============================================================================

#[inline(always)]
fn get_entropy() -> u64 {
    if USE_URANDOM.load(Ordering::Relaxed) {
        return urand::rand();
    }
    get_other_entropy()
}

// Hardware entropy: rdtsc / cycle counter
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn get_other_entropy() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn get_other_entropy() -> u64 {
    let cnt: u64;
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) cnt);
    }
    cnt
}

#[cfg(target_arch = "arm")]
#[inline(always)]
fn get_other_entropy() -> u64 {
    unsafe { core::arch::arm::__pmccntr64() }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
#[inline(always)]
fn get_other_entropy() -> u64 {
    urand::rand() // No hardware source, use urand pool
}

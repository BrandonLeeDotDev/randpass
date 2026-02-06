//! Hardware entropy sources.

#[cfg(target_arch = "x86_64")]
pub fn source_name() -> &'static str {
    "rdtsc"
}

#[cfg(target_arch = "aarch64")]
pub fn source_name() -> &'static str {
    "cycle counter"
}

#[cfg(target_arch = "arm")]
pub fn source_name() -> &'static str {
    "cycle counter"
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
pub fn source_name() -> &'static str {
    "/dev/urandom"
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn entropy() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
pub fn entropy() -> u64 {
    let cnt: u64;
    unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) cnt) }
    cnt
}

#[cfg(target_arch = "arm")]
#[inline(always)]
pub fn entropy() -> u64 {
    unsafe { core::arch::arm::__pmccntr64() }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
#[inline(always)]
pub fn entropy() -> u64 {
    super::urand::rand()
}

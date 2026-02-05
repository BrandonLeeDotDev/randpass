//! Urandom pool - optional /dev/urandom entropy source via 32MB pooled buffer.

#![allow(dead_code)]

use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use zeroize::Zeroize;

use crate::cli::prompts;

const POOL_SIZE: usize = 32 * 1024 * 1024; // 32MB
const POOL_MASK: usize = POOL_SIZE - 1;
const CHUNK_SIZE: usize = 512 * 1024; // 512KB chunks

static mut POOL: *mut u8 = std::ptr::null_mut();
static READ_POS: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static DECLINED: AtomicBool = AtomicBool::new(false);

// =============================================================================
// Public API
// =============================================================================

pub fn is_available() -> bool {
    std::path::Path::new("/dev/urandom").exists()
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Enable urandom pool mode (initializes 32MB pool).
/// Returns false if unavailable or user declined.
pub fn enable() -> bool {
    is_available() && init()
}

pub fn disable() {
    shutdown()
}

/// Returns a random u64 from the pool.
#[inline(always)]
pub fn rand() -> u64 {
    if !ACTIVE.load(Ordering::Relaxed) && (DECLINED.load(Ordering::Relaxed) || !init()) {
        return 0;
    }

    let p = READ_POS.fetch_add(8, Ordering::Relaxed);
    let pos = p.wrapping_add(p >> 25) & POOL_MASK;

    unsafe { std::ptr::read_unaligned(POOL.add(pos) as *const u64) }
}

/// Emergency zero for signal handlers - minimal, async-signal-safe.
#[inline(never)]
pub unsafe fn emergency_zero() {
    unsafe {
        let ptr = POOL;
        if !ptr.is_null() {
            let ptr64 = ptr as *mut u64;
            let count = POOL_SIZE / 8;
            for i in 0..count {
                std::ptr::write_volatile(ptr64.add(i), 0u64);
            }
        }
    }
}

// =============================================================================
// Pool management
// =============================================================================

#[cold]
#[inline(never)]
fn init() -> bool {
    if ACTIVE.load(Ordering::Acquire) {
        return true;
    }
    if DECLINED.load(Ordering::Acquire) {
        return false;
    }

    let layout =
        std::alloc::Layout::from_size_align(POOL_SIZE, 4096).expect("invalid layout constants");
    let pool_ptr = unsafe { std::alloc::alloc(layout) };

    if pool_ptr.is_null() {
        panic!("urand: failed to allocate 32MB pool");
    }

    let mlock_failed = unsafe { libc::mlock(pool_ptr as *const libc::c_void, POOL_SIZE) != 0 };

    if mlock_failed {
        prompts::mlock_failed();

        if !prompts::mlock_continue_prompt() {
            unsafe { std::alloc::dealloc(pool_ptr, layout) };
            DECLINED.store(true, Ordering::Release);
            return false;
        }
    }

    let mut file = File::open("/dev/urandom").expect("urand: failed to open /dev/urandom");
    unsafe {
        file.read_exact(std::slice::from_raw_parts_mut(pool_ptr, POOL_SIZE))
            .expect("urand: failed to read from /dev/urandom");
        POOL = pool_ptr;
    }

    SHUTDOWN.store(false, Ordering::Release);
    READ_POS.store(0, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);

    thread::spawn(move || {
        let mut file = match File::open("/dev/urandom") {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut write_pos = 0usize;

        while !SHUTDOWN.load(Ordering::Relaxed) {
            unsafe {
                let ptr = POOL;
                if ptr.is_null() {
                    break;
                }
                let slice = std::slice::from_raw_parts_mut(ptr.add(write_pos), CHUNK_SIZE);
                let _ = file.read_exact(slice);
            }
            write_pos = (write_pos + CHUNK_SIZE) & POOL_MASK;
            thread::sleep(Duration::from_millis(1000));
        }
    });

    true
}

fn shutdown() {
    if !ACTIVE.load(Ordering::Acquire) {
        return;
    }

    SHUTDOWN.store(true, Ordering::Release);
    thread::sleep(Duration::from_millis(5));

    unsafe {
        let ptr = POOL;
        if !ptr.is_null() {
            POOL = std::ptr::null_mut();
            std::slice::from_raw_parts_mut(ptr, POOL_SIZE).zeroize();
            libc::munlock(ptr as *const libc::c_void, POOL_SIZE);
            let layout = std::alloc::Layout::from_size_align(POOL_SIZE, 4096)
                .expect("invalid layout constants");
            std::alloc::dealloc(ptr, layout);
        }
    }

    ACTIVE.store(false, Ordering::Release);
}

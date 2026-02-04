//! urand - Fast `/dev/urandom` access via a 32MB pooled buffer with background refresh.

#![allow(dead_code)]

use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

const POOL_SIZE: usize = 32 * 1024 * 1024; // 32MB
const POOL_MASK: usize = POOL_SIZE - 1;
const CHUNK_SIZE: usize = 512 * 1024; // 512KB chunks

/// Pool pointer. Written during init/cleanup, read during operation.
static mut POOL: *mut u8 = std::ptr::null_mut();

/// Global position counter for reading from pool.
static READ_POS: AtomicUsize = AtomicUsize::new(0);

/// Signal for background thread shutdown.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Whether pool is currently active.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Initialize the pool and start background refresh thread.
#[cold]
#[inline(never)]
pub fn init() {
    if ACTIVE.load(Ordering::Acquire) {
        return;
    }

    // Allocate 32MB page-aligned buffer
    let layout = std::alloc::Layout::from_size_align(POOL_SIZE, 4096)
        .expect("invalid layout constants");
    let pool_ptr = unsafe { std::alloc::alloc(layout) };

    if pool_ptr.is_null() {
        panic!("urand: failed to allocate 32MB pool");
    }

    // Fill pool from /dev/urandom
    let mut file = File::open("/dev/urandom").expect("urand: failed to open /dev/urandom");
    unsafe {
        file.read_exact(std::slice::from_raw_parts_mut(pool_ptr, POOL_SIZE))
            .expect("urand: failed to read from /dev/urandom");
        POOL = pool_ptr;
    }

    SHUTDOWN.store(false, Ordering::Release);
    READ_POS.store(0, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);

    // Background thread continuously refreshes pool
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
}

/// Shutdown the background thread and free the pool memory.
pub fn shutdown() {
    if !ACTIVE.load(Ordering::Acquire) {
        return;
    }

    // Signal thread to stop
    SHUTDOWN.store(true, Ordering::Release);

    // Give thread time to exit
    thread::sleep(Duration::from_millis(5));

    // Free memory
    unsafe {
        let ptr = POOL;
        if !ptr.is_null() {
            POOL = std::ptr::null_mut();
            let layout = std::alloc::Layout::from_size_align(POOL_SIZE, 4096)
                .expect("invalid layout constants");
            std::alloc::dealloc(ptr, layout);
        }
    }

    ACTIVE.store(false, Ordering::Release);
}

/// Check if urand is currently active.
#[inline]
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Returns a random u64 from the pool.
#[inline(always)]
pub fn rand() -> u64 {
    if !ACTIVE.load(Ordering::Relaxed) {
        init();
    }

    let p = READ_POS.fetch_add(8, Ordering::Relaxed);

    // Lap offset: shift by 1 byte per 32MB to create new byte combinations on wrap
    let pos = p.wrapping_add(p >> 25) & POOL_MASK;

    unsafe { std::ptr::read_unaligned(POOL.add(pos) as *const u64) }
}

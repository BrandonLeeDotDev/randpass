//! RNG test binary - outputs random bytes to stdout for statistical testing.
//!
//! Usage:
//!   ./rng_test              # Use hardware entropy (rdtsc/cycle counter)
//!   ./rng_test --urandom    # Use /dev/urandom pool
//!
//! Pipe to test suites:
//!   ./rng_test | dieharder -a -g 200
//!   ./rng_test | RNG_test stdin -tlmax 1TB
//!   ./rng_test | ./bigcrush_wrapper

use std::io::{self, Write};

mod rand_inline {
    // Inline the RNG to avoid module path issues in bin target
    include!("../src/rand/primes.rs");

    use core::cell::UnsafeCell;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::LazyLock;
    use std::fs::File;
    use std::io::Read;
    use std::thread;
    use std::time::Duration;

    // =========================================================================
    // Urandom pool
    // =========================================================================

    const POOL_SIZE: usize = 32 * 1024 * 1024;
    const POOL_MASK: usize = POOL_SIZE - 1;
    const CHUNK_SIZE: usize = 512 * 1024;

    static mut POOL: *mut u8 = std::ptr::null_mut();
    static READ_POS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWN: AtomicBool = AtomicBool::new(false);
    static POOL_ACTIVE: AtomicBool = AtomicBool::new(false);

    pub fn init_urandom() {
        if POOL_ACTIVE.load(Ordering::Acquire) {
            return;
        }

        let layout = std::alloc::Layout::from_size_align(POOL_SIZE, 4096)
            .expect("invalid layout");
        let pool_ptr = unsafe { std::alloc::alloc(layout) };

        if pool_ptr.is_null() {
            panic!("failed to allocate pool");
        }

        let mut file = File::open("/dev/urandom").expect("failed to open /dev/urandom");
        unsafe {
            file.read_exact(std::slice::from_raw_parts_mut(pool_ptr, POOL_SIZE))
                .expect("failed to read /dev/urandom");
            POOL = pool_ptr;
        }

        SHUTDOWN.store(false, Ordering::Release);
        READ_POS.store(0, Ordering::Release);
        POOL_ACTIVE.store(true, Ordering::Release);

        thread::spawn(move || {
            let mut file = match File::open("/dev/urandom") {
                Ok(f) => f,
                Err(_) => return,
            };
            let mut write_pos = 0usize;

            while !SHUTDOWN.load(Ordering::Relaxed) {
                unsafe {
                    let ptr = POOL;
                    if ptr.is_null() { break; }
                    let slice = std::slice::from_raw_parts_mut(ptr.add(write_pos), CHUNK_SIZE);
                    let _ = file.read_exact(slice);
                }
                write_pos = (write_pos + CHUNK_SIZE) & POOL_MASK;
                thread::sleep(Duration::from_millis(1000));
            }
        });
    }

    #[inline(always)]
    fn urandom_rand() -> u64 {
        let p = READ_POS.fetch_add(8, Ordering::Relaxed);
        let pos = p.wrapping_add(p >> 25) & POOL_MASK;
        unsafe { std::ptr::read_unaligned(POOL.add(pos) as *const u64) }
    }

    // =========================================================================
    // Main RNG
    // =========================================================================

    static USE_URANDOM: AtomicBool = AtomicBool::new(false);

    pub fn set_urandom_mode(enabled: bool) {
        if enabled {
            init_urandom();
        }
        USE_URANDOM.store(enabled, Ordering::SeqCst);
    }

    static RAND: LazyLock<Rand> = LazyLock::new(Rand::new);
    pub struct Rand(UnsafeCell<usize>);
    unsafe impl Sync for Rand {}

    impl Rand {
        pub fn new() -> Self {
            Rand(UnsafeCell::new(get_entropy() as usize))
        }
    }

    #[inline(always)]
    pub fn get() -> usize {
        let state = unsafe { *RAND.0.get() };
        let entropy = get_entropy() as usize;

        let mixed = state ^ entropy;
        let idx = (mixed ^ (mixed >> 32)) as usize % PRIMES.len();

        let new_state = state
            .rotate_left(17)
            .wrapping_mul(PRIMES[idx])
            ^ entropy;

        unsafe { *RAND.0.get() = new_state; }

        let mut z = new_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9_usize);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb_usize);
        z ^ (z >> 31)
    }

    #[inline(always)]
    fn get_entropy() -> u64 {
        if USE_URANDOM.load(Ordering::Relaxed) {
            return urandom_rand();
        }
        get_hw_entropy()
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    fn get_hw_entropy() -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }

    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    fn get_hw_entropy() -> u64 {
        let cnt: u64;
        unsafe { core::arch::asm!("mrs {}, cntvct_el0", out(reg) cnt); }
        cnt
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    #[inline(always)]
    fn get_hw_entropy() -> u64 {
        init_urandom();
        urandom_rand()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let use_urandom = args.iter().any(|a| a == "--urandom" || a == "-u");

    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: rng_test [OPTIONS]");
        eprintln!();
        eprintln!("Outputs random bytes to stdout for statistical testing.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -u, --urandom  Use /dev/urandom pool instead of hardware entropy");
        eprintln!("  -h, --help     Show this help");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  rng_test | dieharder -a -g 200");
        eprintln!("  rng_test | RNG_test stdin -tlmax 1TB");
        eprintln!("  rng_test | ./bigcrush_wrapper --big");
        eprintln!();
        eprintln!("Run both entropy sources in parallel:");
        eprintln!("  rng_test | ./bigcrush_wrapper -n 'HW' > hw.txt 2>&1 &");
        eprintln!("  rng_test -u | ./bigcrush_wrapper -n 'urand' > urand.txt 2>&1 &");
        std::process::exit(0);
    }

    rand_inline::set_urandom_mode(use_urandom);

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut buf = [0u8; 8192];

    loop {
        for chunk in buf.chunks_exact_mut(8) {
            let val = rand_inline::get();
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        if out.write_all(&buf).is_err() {
            break;
        }
    }
}

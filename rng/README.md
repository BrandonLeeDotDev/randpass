# RNG Testing

Statistical test suite for validating the randpass random number generator.

## Test Results Summary

| Suite | Tests | HW (rdtsc) | urandom |
|-------|-------|------------|---------|
| BigCrush | 160 | 159 pass, 1 weak | 158 pass, 2 weak |
| Dieharder | 114 | 112 pass, 2 weak | 112 pass, 2 weak |

Both entropy sources pass all tests. "Weak" results are borderline p-values that would likely pass on re-run.

## Building

```bash
# Build randpass
cargo build --release

# Build the BigCrush wrapper (requires TestU01)
gcc -O3 -o bigcrush_wrapper rng/bigcrush_wrapper.c -ltestu01 -lprobdist -lmylib -lm
```

## Running Tests

### Dieharder

Full battery (~30 minutes):
```bash
randpass --bytes | dieharder -a -g 200 | tee rng/results/hw_dieharder.txt
randpass --bytes -u | dieharder -a -g 200 | tee rng/results/urand_dieharder.txt
```

With unbuffered output for real-time progress:
```bash
stdbuf -oL randpass --bytes | stdbuf -oL dieharder -a -g 200 2>&1 | tee rng/results/hw_dieharder.txt &
stdbuf -oL randpass --bytes -u | stdbuf -oL dieharder -a -g 200 2>&1 | tee rng/results/urand_dieharder.txt &
```

### TestU01 BigCrush

Requires TestU01 library installed. See install instructions below.

SmallCrush (~10 seconds):
```bash
randpass --bytes | ./bigcrush_wrapper --small
```

Crush (~30 minutes):
```bash
randpass --bytes | ./bigcrush_wrapper --medium
```

BigCrush (~4 hours):
```bash
stdbuf -oL randpass --bytes | stdbuf -oL ./bigcrush_wrapper -n 'rdtsc' 2>&1 | tee rng/results/hw_bigcrush.txt &
stdbuf -oL randpass --bytes -u | stdbuf -oL ./bigcrush_wrapper -n 'urandom' 2>&1 | tee rng/results/urand_bigcrush.txt &
```

### PractRand

Runs until failure (can take hours to days):
```bash
randpass --bytes | RNG_test stdin -tlmax 1TB
```

## Installing TestU01

```bash
wget http://simul.iro.umontreal.ca/testu01/TestU01.zip
unzip TestU01.zip && cd TestU01-1.2.3
./configure --prefix=/usr/local
make && sudo make install
sudo ldconfig
```

## Entropy Sources

- **Hardware (default)**: Uses CPU timestamp counter (`rdtsc` on x86_64, `cntvct_el0` on ARM)
- **urandom (`-u` flag)**: Uses 32MB pooled buffer from `/dev/urandom` with background refresh

## Files

| File | Description |
|------|-------------|
| `bigcrush_wrapper.c` | C wrapper for TestU01 BigCrush battery |
| `results/hw_bigcrush.txt` | BigCrush results for hardware entropy |
| `results/urand_bigcrush.txt` | BigCrush results for urandom entropy |
| `results/hw_dieharder.txt` | Dieharder results for hardware entropy |
| `results/urand_dieharder.txt` | Dieharder results for urandom entropy |

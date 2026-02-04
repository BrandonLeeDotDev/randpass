/*
 * TestU01 BigCrush wrapper for randpass RNG
 *
 * This reads random bytes from stdin and feeds them to TestU01's BigCrush battery.
 *
 * Build:
 *   gcc -O3 -o bigcrush_wrapper bigcrush_wrapper.c -ltestu01 -lprobdist -lmylib -lm
 *
 * Usage:
 *   cargo build --release --bin rng_test
 *   ./target/release/rng_test | ./bigcrush_wrapper
 *
 * Install TestU01:
 *   wget http://simul.iro.umontreal.ca/testu01/TestU01.zip
 *   unzip TestU01.zip && cd TestU01-1.2.3
 *   ./configure --prefix=/usr/local
 *   make && sudo make install
 *   sudo ldconfig
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "unif01.h"
#include "bbattery.h"

#define BUFFER_SIZE 4096

static uint8_t buffer[BUFFER_SIZE];
static size_t buffer_pos = BUFFER_SIZE;  /* Start empty to trigger first read */
static size_t buffer_len = 0;

static int refill_buffer(void) {
    buffer_len = fread(buffer, 1, BUFFER_SIZE, stdin);
    buffer_pos = 0;
    return buffer_len > 0;
}

static uint32_t read_uint32(void) {
    uint32_t value = 0;

    for (int i = 0; i < 4; i++) {
        if (buffer_pos >= buffer_len) {
            if (!refill_buffer()) {
                fprintf(stderr, "Error: stdin exhausted\n");
                exit(1);
            }
        }
        value |= ((uint32_t)buffer[buffer_pos++]) << (i * 8);
    }

    return value;
}

/* TestU01 generator function - returns unsigned 32-bit integer */
static unsigned long stdin_bits(void *param, void *state) {
    (void)param;
    (void)state;
    return (unsigned long)read_uint32();
}

/* TestU01 generator function - returns double in [0, 1) */
static double stdin_u01(void *param, void *state) {
    (void)param;
    (void)state;
    return read_uint32() / 4294967296.0;
}

/* Write generator state (no-op for stdin) */
static void write_state(void *state) {
    (void)state;
    printf("\n");
}

static unif01_Gen *create_stdin_gen(const char *name) {
    unif01_Gen *gen = malloc(sizeof(unif01_Gen));
    if (!gen) {
        fprintf(stderr, "Error: malloc failed\n");
        exit(1);
    }

    gen->state = NULL;
    gen->param = NULL;
    gen->name = strdup(name);
    gen->GetU01 = stdin_u01;
    gen->GetBits = stdin_bits;
    gen->Write = write_state;

    return gen;
}

static void delete_stdin_gen(unif01_Gen *gen) {
    if (gen) {
        free((void *)gen->name);
        free(gen);
    }
}

int main(int argc, char *argv[]) {
    const char *name = "Rust RNG";
    int test_type = 2;  /* Default: BigCrush */

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-s") == 0 || strcmp(argv[i], "--small") == 0) {
            test_type = 0;
        } else if (strcmp(argv[i], "-m") == 0 || strcmp(argv[i], "--medium") == 0) {
            test_type = 1;
        } else if (strcmp(argv[i], "-b") == 0 || strcmp(argv[i], "--big") == 0) {
            test_type = 2;
        } else if (strcmp(argv[i], "-n") == 0 || strcmp(argv[i], "--name") == 0) {
            if (i + 1 < argc) {
                name = argv[++i];
            }
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Usage: %s [OPTIONS]\n", argv[0]);
            printf("\nReads random bytes from stdin and runs TestU01 battery.\n");
            printf("\nOptions:\n");
            printf("  -s, --small   Run SmallCrush (~10 seconds)\n");
            printf("  -m, --medium  Run Crush (~30 minutes)\n");
            printf("  -b, --big     Run BigCrush (~4 hours) [default]\n");
            printf("  -n, --name    Generator name for report\n");
            printf("  -h, --help    Show this help\n");
            printf("\nExample:\n");
            printf("  ./target/release/rng_test | %s --small\n", argv[0]);
            return 0;
        }
    }

    unif01_Gen *gen = create_stdin_gen(name);

    switch (test_type) {
        case 0:
            printf("Running SmallCrush on '%s'...\n", name);
            bbattery_SmallCrush(gen);
            break;
        case 1:
            printf("Running Crush on '%s'...\n", name);
            bbattery_Crush(gen);
            break;
        case 2:
            printf("Running BigCrush on '%s'...\n", name);
            bbattery_BigCrush(gen);
            break;
    }

    delete_stdin_gen(gen);
    return 0;
}

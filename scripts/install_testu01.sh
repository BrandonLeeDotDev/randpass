#!/bin/bash
# Install TestU01 statistical test library
# http://simul.iro.umontreal.ca/testu01/tu01.html

set -e

TESTU01_VERSION="1.2.3"
INSTALL_PREFIX="${1:-/usr/local}"

echo "Installing TestU01 ${TESTU01_VERSION} to ${INSTALL_PREFIX}..."

cd /tmp
rm -rf TestU01* testu01*

echo "Downloading TestU01..."
wget -q "http://simul.iro.umontreal.ca/testu01/TestU01.zip"
unzip -q TestU01.zip
cd "TestU01-${TESTU01_VERSION}"

echo "Configuring..."
./configure --prefix="${INSTALL_PREFIX}"

echo "Building (this takes a few minutes)..."
make -j$(nproc)

echo "Installing (may require sudo)..."
if [ "$INSTALL_PREFIX" = "/usr/local" ]; then
    sudo make install
    sudo ldconfig
else
    make install
fi

echo ""
echo "TestU01 installed successfully!"
echo ""
echo "Build the BigCrush wrapper:"
echo "  cd /path/to/randpass"
echo "  gcc -O3 -o bigcrush_wrapper src/bin/bigcrush_wrapper.c -ltestu01 -lprobdist -lmylib -lm"
echo ""
echo "Run tests:"
echo "  ./target/release/rng_test | ./bigcrush_wrapper --small   # ~10 seconds"
echo "  ./target/release/rng_test | ./bigcrush_wrapper --medium  # ~30 minutes"
echo "  ./target/release/rng_test | ./bigcrush_wrapper --big     # ~4 hours"

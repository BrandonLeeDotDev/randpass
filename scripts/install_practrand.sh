#!/bin/bash
# Install PractRand from source
# PractRand is the most stringent RNG test suite available

set -e

PRACTRAND_VERSION="0.94"
INSTALL_DIR="${HOME}/.local"

echo "Installing PractRand ${PRACTRAND_VERSION}..."

cd /tmp

# Clean up any previous attempts
rm -rf PractRand-* practrand-*

# Download from SourceForge (may require manual download if this fails)
echo "Downloading from SourceForge..."
if ! curl -L "https://downloads.sourceforge.net/project/pracrand/PractRand-${PRACTRAND_VERSION}.tar.gz" -o "PractRand-${PRACTRAND_VERSION}.tar.gz" 2>/dev/null; then
    echo ""
    echo "Automatic download failed. Please download manually:"
    echo "  https://sourceforge.net/projects/pracrand/files/PractRand-${PRACTRAND_VERSION}.tar.gz"
    echo ""
    echo "Then run: tar -xzf PractRand-${PRACTRAND_VERSION}.tar.gz && cd PractRand-${PRACTRAND_VERSION}"
    echo "And: g++ -O3 -std=c++11 -pthread tools/RNG_test.cpp src/*.cpp -Iinclude -o RNG_test"
    exit 1
fi

# Check if it's actually a tarball
if ! file "PractRand-${PRACTRAND_VERSION}.tar.gz" | grep -q "gzip"; then
    echo "Downloaded file is not a valid tarball (SourceForge redirect issue)."
    echo "Please download manually from:"
    echo "  https://sourceforge.net/projects/pracrand/files/PractRand-${PRACTRAND_VERSION}.tar.gz"
    exit 1
fi

tar -xzf "PractRand-${PRACTRAND_VERSION}.tar.gz"
cd "PractRand-${PRACTRAND_VERSION}"

echo "Building PractRand..."
g++ -O3 -std=c++11 -pthread tools/RNG_test.cpp src/*.cpp -Iinclude -o RNG_test

echo "Installing to ${INSTALL_DIR}/bin..."
mkdir -p "${INSTALL_DIR}/bin"
cp RNG_test "${INSTALL_DIR}/bin/"

echo ""
echo "PractRand installed successfully!"
echo "Add ${INSTALL_DIR}/bin to your PATH if not already present:"
echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
echo ""
echo "Usage: ./target/release/rng_test | RNG_test stdin -tlmax 1TB"

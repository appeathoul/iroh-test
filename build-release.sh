#!/bin/bash

# Release script - Create release package with images

set -e

echo "🚀 Starting release build..."

# Clean previous builds
cargo clean

# Build release version
cargo build --release

# Create release directory
RELEASE_DIR="release-package"
rm -rf $RELEASE_DIR
mkdir -p $RELEASE_DIR

# Copy executable file
cp target/release/iroh-test $RELEASE_DIR/

# Copy images directory
cp -r target/release/images $RELEASE_DIR/

# Copy README and other documentation
cp README.md $RELEASE_DIR/
if [ -f "INPUT_LISTENING.md" ]; then
    cp INPUT_LISTENING.md $RELEASE_DIR/
fi

echo "✅ Release package created in $RELEASE_DIR directory"
echo "📁 Included files:"
ls -la $RELEASE_DIR/
echo ""
echo "📁 Image files:"
ls -la $RELEASE_DIR/images/
echo ""
echo "🎉 You can now distribute the entire $RELEASE_DIR directory to users"
echo "💡 Users can run the application with the following command:"
echo "   cd $RELEASE_DIR && ./iroh-test --help"
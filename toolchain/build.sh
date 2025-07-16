#!/bin/bash
set -e

echo "Setting up Twizzler development environment..."

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    echo "Error: /twizzler is not a git repository"
    echo "Make sure you're mounting the Twizzler git repo to /twizzler"
    exit 1
fi

# Update submodules
echo "Updating git submodules..."
git submodule update --init --recursive

# Run cargo bootstrap
echo "Running cargo bootstrap..."
cargo bootstrap

echo "Bootstrapping Complete!"

echo "Starting Pruning"
cd toolchain
while read -r path; do rm -rf "$path"; done < ./to_remove.txt

echo "Pruning Complete"
echo "Toolchain build complete!"

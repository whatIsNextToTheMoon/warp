#!/usr/bin/env bash

set -e

BUILD_ARCH="$3"
BUILD_ARCH="${BUILD_ARCH:-$(uname -m)}"

# Update packages together with cached package lists, retaining the Dockerfile's fakeroot workaround.
sudo pacman -Syu --noconfirm --ignore fakeroot

# Run the bundle script, specifying the release channel and tag, skipping
# building the binary (as we have already done so), and only bundling the
# Arch package.
./script/bundle --channel $1 --release-tag $2 --skip-build --packages arch --arch $BUILD_ARCH --artifact $4

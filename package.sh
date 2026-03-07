#!/bin/bash
set -e

# If we are in the toolchain container, ensure we are in the right directory
if [ -d "/build" ]; then
    cd /build
fi

# Ensure we are in the project root (where this script is)
cd "$(dirname "$0")"

echo "--- Starting Optimized Incremental Build ---"

# 1. Build Binary (Uses mounted target/ for instant incremental build)
echo "Building Release Binary..."
cd waywidget
cargo build --release
cd ..
mkdir -p dest/bin
cp waywidget/target/release/waywidget dest/bin/

# 2. Build RPM (Re-uses the release binary we just built)
echo "Building RPM..."
mkdir -p ~/rpmbuild/{SOURCES,SPECS,RPMS,SRPMS,BUILD,BUILDROOT}
# We create a minimal tarball just for the spec's sake
tar -czf ~/rpmbuild/SOURCES/waywidget-0.1.11.tar.gz --exclude='./waywidget/target' --transform 's,^,waywidget-0.1.11/,' .
cp packaging/waywidget.spec ~/rpmbuild/SPECS/
rpmbuild -ba ~/rpmbuild/SPECS/waywidget.spec
cp ~/rpmbuild/RPMS/x86_64/*.rpm dest/

# 3. Build Flatpak (Uses mounted .flatpak-builder/ for caching)
# echo "Building Flatpak..."
# flatpak-builder --force-clean --disable-rofiles-fuse \
#     --share-network \
#     --ccache \
#     --repo=dest/repo build-dir packaging/org.example.WayWidget.yaml
# flatpak build-bundle dest/repo dest/waywidget.flatpak org.example.WayWidget

echo "--- Build Complete! ---"

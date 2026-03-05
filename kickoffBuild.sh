#!/bin/bash
set -e

# Run version update
./updateVersion.sh "$1"

# Extract the new version
NEW_VERSION=$(grep '^version =' waywidget/Cargo.toml | head -n 1 | cut -d '"' -f 2)

# Git operations
git add waywidget/Cargo.toml packaging/waywidget.spec package.sh
git commit -m "chore: Bump version to $NEW_VERSION"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"
git push origin main
git push origin "v$NEW_VERSION"

echo "Build kicked off for version v$NEW_VERSION"

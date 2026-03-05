#!/bin/bash
set -e

# Extract current version from Cargo.toml
CURRENT_VERSION=$(grep '^version =' waywidget/Cargo.toml | head -n 1 | cut -d '"' -f 2)
echo "Current version: $CURRENT_VERSION"

# Default to incrementing patch version if no argument provided
if [ -z "$1" ]; then
    MAJOR=$(echo $CURRENT_VERSION | cut -d. -f1)
    MINOR=$(echo $CURRENT_VERSION | cut -d. -f2)
    PATCH=$(echo $CURRENT_VERSION | cut -d. -f3)
    NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
else
    NEW_VERSION=$1
fi

echo "New version: $NEW_VERSION"

# Update Cargo.toml
sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" waywidget/Cargo.toml

# Update waywidget.spec
sed -i "s/^Version:        $CURRENT_VERSION/Version:        $NEW_VERSION/" packaging/waywidget.spec

# Update package.sh
sed -i "s/waywidget-$CURRENT_VERSION/waywidget-$NEW_VERSION/g" package.sh

echo "Version updated successfully to $NEW_VERSION"

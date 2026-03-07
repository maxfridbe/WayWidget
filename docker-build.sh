#!/bin/bash
set -e

# Detect container tool (prefer podman)
CONTAINER_TOOL=$(command -v podman || command -v podman-remote || command -v docker)

if [ -z "$CONTAINER_TOOL" ]; then
    echo "Error: Neither podman nor docker found in PATH."
    exit 1
fi

IMAGE_NAME="waywidget-toolchain"

echo "Using tool: $CONTAINER_TOOL"

# 1. Build the toolchain image
echo "--- Building Toolchain Image ---"
$CONTAINER_TOOL build -t $IMAGE_NAME .

# 2. Run the packaging script inside the container
echo "--- Running Build in Container ---"
$CONTAINER_TOOL run --rm \
    --security-opt label=disable \
    --security-opt seccomp=unconfined \
    -v "$(pwd)":/build:Z \
    $IMAGE_NAME

echo "--- Build Finished! Artifacts are in ./dest ---"

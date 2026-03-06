#!/bin/bash

# Ensure Rust is available
source $HOME/.cargo/env

# Build the project
cd waywidget
cargo build
cd ..

# Function to run example
run_example() {
    local name=$1
    local svg=$2
    local script=$3
    local w=$4
    local h=$5
    echo "Running example: $name"
    local cmd="./waywidget/target/debug/waywidget --svg $svg --width $w --height $h"
    if [ -n "$script" ]; then
        cmd="$cmd --script $script"
    fi
    $cmd
}

# Select example
EXAMPLE=${1:-lcars}

case $EXAMPLE in
    "clock")
        run_example "Analog Clock" "examples/clock/clock.svg" "examples/clock/widget.js" 200 200
        ;;
    "lcars")
        run_example "LCARS Clock" "examples/lcars_clock/widget.svg" "examples/lcars_clock/widget.js" 600 300
        ;;
    "sunrise")
        run_example "Sunrise Cycle" "examples/sunrise/widget.svg" "examples/sunrise/widget.js" 800 450
        ;;
    "lion")
        run_example "Static Lion" "examples/lion/widget.svg" "" 200 200
        ;;
    *)
        echo "Usage: ./run.sh [clock|lcars|sunrise|lion]"
        ;;
esac

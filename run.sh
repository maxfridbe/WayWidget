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
    local u=$6
    echo "Running example: $name"
    ./waywidget/target/debug/waywidget --svg "$svg" --script "$script" --width "$w" --height "$h" --updateS "$u"
}

# Select example
EXAMPLE=${1:-lcars}

case $EXAMPLE in
    "clock")
        run_example "Analog Clock" "examples/clock/clock.svg" "examples/clock/widget.js" 200 200 1.0
        ;;
    "lcars")
        run_example "LCARS Clock" "examples/lcars_clock/widget.svg" "examples/lcars_clock/widget.js" 600 300 1.0
        ;;
    "sunrise")
        run_example "Sunrise Cycle" "examples/sunrise/widget.svg" "examples/sunrise/widget.js" 800 450 0.033
        ;;
    *)
        echo "Usage: ./run.sh [clock|lcars]"
        ;;
esac

#!/bin/bash

# Ensure Rust is available
source $HOME/.cargo/env

# Build the project
cd waywidget
cargo build
cd ..

# Function to run example using the new 'run' command
# Note: This expects the widgets to be in ~/.config/waywidget/
run_widget() {
    local widget=$1
    local name=$2
    echo "Running widget: $widget (as $name)"
    ./waywidget/target/debug/waywidget run "$widget" --name "$name"
}

# Setup convention if needed (for demonstration)
CONFIG_DIR="$HOME/.config/waywidget"
mkdir -p "$CONFIG_DIR/clock"
mkdir -p "$CONFIG_DIR/sunrise"
cp examples/clock/clock.svg "$CONFIG_DIR/clock/widget.svg"
cp examples/clock/widget.js "$CONFIG_DIR/clock/widget.js"
cp examples/sunrise/widget.svg "$CONFIG_DIR/sunrise/widget.svg"
cp examples/sunrise/widget.js "$CONFIG_DIR/sunrise/widget.js"

# Select example
EXAMPLE=${1:-lcars}

case $EXAMPLE in
    "clock")
        run_widget "clock" "myclock"
        ;;
    "sunrise")
        run_widget "sunrise" "morning"
        ;;
    "run")
        # Direct run of a widget name
        run_widget "$2" "${3:-$2}"
        ;;
    *)
        echo "Usage: ./run.sh [clock|sunrise|run <name>]"
        echo "Example: ./run.sh run clock"
        ;;
esac

#!/bin/bash

# Ensure Rust is available
source $HOME/.cargo/env

# Build the project
cd waywidget
cargo build
cd ..

BINARY="./waywidget/target/debug/waywidget"

# Check for desktop flag as second argument
DESKTOP_ARG=""
if [ "$2" == "--desktop" ]; then
    DESKTOP_ARG="--desktop"
fi

# Function to run example
run_example() {
    local name=$1
    local svg=$2
    local script=$3
    local w=$4
    local h=$5
    echo "Running example: $name"
    local cmd="$BINARY --svg $svg --width $w --height $h $DESKTOP_ARG"
    if [ -n "$script" ]; then
        cmd="$cmd --script $script"
    fi
    $cmd
}

# Select example
EXAMPLE=${1:-lcars}

case $EXAMPLE in
    "all")
        echo "Launching all widgets..."
        # Left side
        $BINARY --svg examples/lcars_clock/widget.svg --script examples/lcars_clock/widget.js --width 600 --height 300 --position 50,50 $DESKTOP_ARG &
        PID1=$!
        $BINARY --svg examples/ip_visualizer/widget.svg --script examples/ip_visualizer/widget.js --width 350 --height 200 --position 50,400 $DESKTOP_ARG &
        PID2=$!
        
        # Middle
        $BINARY --svg examples/sunrise/widget.svg --script examples/sunrise/widget.js --width 800 --height 450 --position 700,50 $DESKTOP_ARG &
        PID3=$!
        $BINARY --svg examples/clock/clock.svg --script examples/clock/widget.js --width 200 --height 200 --position 700,550 $DESKTOP_ARG &
        PID4=$!

        # Right side
        $BINARY --svg examples/keyboard/widget.svg --script examples/keyboard/widget.js --width 820 --height 350 --position 1550,50 $DESKTOP_ARG &
        PID5=$!
        $BINARY --svg examples/warpcore/widget.svg --script examples/warpcore/widget.js --width 150 --height 400 --position 1550,450 $DESKTOP_ARG &
        PID6=$!

        # Weather
        $BINARY --svg examples/weather/widget.svg --script examples/weather/widget.js --width 700 --height 220 --position 700,800 $DESKTOP_ARG &
        PID7=$!

        trap "kill $PID1 $PID2 $PID3 $PID4 $PID5 $PID6 $PID7; exit" INT TERM
        wait
        ;;
    "weather")
        run_example "Weather Forecast" "examples/weather/widget.svg" "examples/weather/widget.js" 700 220
        ;;
    "calculator")
        run_example "Calculator" "examples/calculator/widget.svg" "examples/calculator/widget.js" 250 350
        ;;
    "clock")
        run_example "Analog Clock" "examples/clock/clock.svg" "examples/clock/widget.js" 200 200
        ;;
    "lcars")
        run_example "LCARS Clock" "examples/lcars_clock/widget.svg" "examples/lcars_clock/widget.js" 600 300
        ;;
    "sunrise")
        run_example "Sunrise Cycle" "examples/sunrise/widget.svg" "examples/sunrise/widget.js" 800 450
        ;;
    "keyboard")
        run_example "Keyboard Visualizer" "examples/keyboard/widget.svg" "examples/keyboard/widget.js" 820 350
        ;;
    "warpcore")
        run_example "Warp Core" "examples/warpcore/widget.svg" "examples/warpcore/widget.js" 150 400
        ;;
    "ip")
        run_example "IP Visualizer" "examples/ip_visualizer/widget.svg" "examples/ip_visualizer/widget.js" 350 200
        ;;
    "lion")
        run_example "Static Lion" "examples/lion/widget.svg" "" 200 200
        ;;
    *)
        echo "Usage: ./run.sh [all|clock|lcars|sunrise|keyboard|warpcore|ip|lion|calculator] [--desktop]"
        ;;
esac

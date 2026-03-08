#!/bin/bash

# Ensure Rust is available
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# Build the project
echo "Building WayWidget..."
cd waywidget
cargo build --quiet
cd ..

BINARY="./waywidget/target/debug/waywidget"

# Setup config directory if it doesn't exist (copy examples)
CONFIG_DIR="$HOME/.config/waywidget"
if [ ! -d "$CONFIG_DIR" ]; then
    echo "Initializing config directory in $CONFIG_DIR..."
    mkdir -p "$CONFIG_DIR"
    cp -r examples/* "$CONFIG_DIR/"
fi

# Select example
EXAMPLE=${1:-lcars}
SHIFT_ARGS="${@:2}"

case $EXAMPLE in
    "all")
        echo "Launching workspace..."
        $BINARY run lcars_clock --width 600 --height 300 $SHIFT_ARGS &
        $BINARY run ip_visualizer --width 350 --height 200 $SHIFT_ARGS &
        $BINARY run sunrise --width 800 --height 450 $SHIFT_ARGS &
        $BINARY run clock --width 200 --height 200 $SHIFT_ARGS &
        $BINARY run keyboard --width 820 --height 350 $SHIFT_ARGS &
        $BINARY run warpcore --width 150 --height 400 $SHIFT_ARGS &
        $BINARY run weather --width 700 --height 220 $SHIFT_ARGS &
        $BINARY run tailscale --width 300 --height 60 $SHIFT_ARGS &
        
        echo "All widgets launched. Use 'waywidget stop --name <name>' to close them."
        ;;
    "install")
        echo "Updating examples in $CONFIG_DIR..."
        cp -r examples/* "$CONFIG_DIR/"
        ;;
    *)
        # Check if it's a known example
        if [ -d "examples/$EXAMPLE" ]; then
            # Ensure it's in config dir for 'run' command to work
            mkdir -p "$CONFIG_DIR/$EXAMPLE"
            cp -r examples/$EXAMPLE/* "$CONFIG_DIR/$EXAMPLE/"
            
            # Default sizes for known widgets
            WIDTH=200
            HEIGHT=200
            case $EXAMPLE in
                "lcars_clock") WIDTH=600; HEIGHT=300 ;;
                "weather") WIDTH=700; HEIGHT=220 ;;
                "keyboard") WIDTH=820; HEIGHT=350 ;;
                "warpcore") WIDTH=150; HEIGHT=400 ;;
                "ip_visualizer") WIDTH=350; HEIGHT=200 ;;
                "sunrise") WIDTH=800; HEIGHT=450 ;;
                "tailscale") WIDTH=300; HEIGHT=60 ;;
            esac
            
            echo "Running $EXAMPLE..."
            $BINARY run "$EXAMPLE" --width $WIDTH --height $HEIGHT $SHIFT_ARGS
        else
            echo "Usage: ./run.sh [all|install|<widget_name>] [--desktop|--float|--position x,y]"
            echo "Available widgets: $(ls examples/ | grep -v '\.d\.ts' | grep -v '\.svg' | tr '\n' ' ')"
        fi
        ;;
esac

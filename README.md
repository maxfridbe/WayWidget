# WayWidgets System

A lightweight, high-performance Wayland widget system that renders SVG templates using Cairo and provides dynamic logic via an embedded JavaScript engine (Boa).

## Features

- **SVG Rendering**: Uses `librsvg` and `cairo` for crisp, vector-based visuals.
- **JS Scripting**: Dynamic updates via an embedded JavaScript engine.
- **Interactive**: Built-in support for moving and resizing widgets via the Wayland protocol.
- **Generic**: Run any widget by providing an SVG and a JS script.

## Getting Started

### Prerequisites

Ensure you have the following system libraries installed (Debian/Ubuntu):
```bash
sudo apt install libwayland-dev libcairo2-dev librsvg2-dev libxkbcommon-dev pkg-config
```

### Running Examples

Use the provided helper script to run the examples:

```bash
./run.sh lcars   # Star Trek themed digital clock
./run.sh clock   # Standard analog clock
```

## JavaScript Interaction API

The system looks for a global `update()` function in your JavaScript file. This function is called once per second (by default).

### The `update()` Function

Your script must define:
```javascript
function update() {
    // ... logic ...
    return {
        "element-id": 90,          // Rotates element with id="element-id" by 90 degrees
        "text-element": "Hello!"   // Sets text content of element with id="text-element"
    };
}
```

### Return Value Types

The `WayWidget` engine processes the returned object keys as follows:

1.  **Numbers (Rotations)**:
    - If the value is a **Number**, the engine finds the SVG element with that ID and applies a `transform="rotate(value, 50, 50)"`.
    - *Note: The rotation center is currently fixed at 50,50 (midpoint of a 100x100 viewBox).*

2.  **Strings (Text)**:
    - If the value is a **String**, the engine finds the SVG element with that ID and replaces its inner text content with the string.
    - This is ideal for `<text>` elements in digital clocks or status monitors.

### Example: Digital Clock (`widget.js`)

```javascript
function update() {
    const now = new Date();
    return {
        "time-display": now.toLocaleTimeString(),
        "date-display": now.toDateString()
    };
}
```

## Interaction

- **Move**: Left-click and drag anywhere on the widget to move it.
- **Resize**: Hover over the bottom-right corner to reveal the resize handle. Left-click and drag the handle to resize the window. The SVG will automatically scale to fit the new dimensions while maintaining its aspect ratio defined in the `viewBox`.

## CLI Usage

```bash
waywidget --svg <PATH_TO_SVG> --script <PATH_TO_JS> --width <WIDTH> --height <HEIGHT>
```

- `-s, --svg`: Path to the SVG template file.
- `-j, --script`: Path to the JavaScript logic file.
- `--updateS`: Update interval in seconds (default: 0.0, which means no automatic update).
- `--width`: Initial window width (default: 200).
- `--height`: Initial window height (default: 200).

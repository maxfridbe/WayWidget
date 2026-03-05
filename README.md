# WayWidgets System

A lightweight, high-performance Wayland widget system that renders SVG templates using Cairo and provides dynamic logic via an embedded JavaScript engine (Boa).

![Sunrise Example](sunrise.png)

## Features

- **SVG Rendering**: Uses `librsvg` and `cairo` for crisp, vector-based visuals.
- **JS Scripting**: Dynamic updates via a fluent JavaScript API.
- **Interactive**: Built-in support for moving and resizing widgets via the Wayland protocol.
- **Generic**: Run any widget by providing an SVG and a JS script.
- **Multi-format Packaging**: Automated builds for Release Binaries, RPMs, and Flatpaks.

## Getting Started

### Prerequisites

Ensure you have the following system libraries installed (Debian/Ubuntu):
```bash
sudo apt install libwayland-dev libcairo2-dev librsvg2-dev libxkbcommon-dev pkg-config
```

### Running Examples

Use the provided helper script to run the examples:

```bash
./run.sh lcars     # Star Trek themed digital clock
./run.sh clock     # Standard analog clock
./run.sh sunrise   # Animated 60-second day/night cycle
./run.sh lion      # Static geometric lion widget
```

## JavaScript Interaction API

The system looks for a global `update(api, timestamp, click, state)` function.

- `api`: The `WidgetAPI` instance for finding elements and manipulating their attributes.
- `timestamp`: The current time in milliseconds (useful for animations).
- `click`: An object `{ x: number, y: number }` representing normalized coordinates (0.0 to 1.0) of the last click, or `null` if no click occurred in the last frame.
- `state`: A persistent `WidgetState` store that survives between `update` calls.

### Example: Interactive Sunrise (`widget.js`)

```javascript
function update(api, timestamp, click, state) {
    let enabled = state.get("enabled") || "true";

    if (click) {
        enabled = (enabled === "true") ? "false" : "true";
        state.set("enabled", enabled);
        console.log("Animation enabled:", enabled);
    }

    // Rich state saving with JSON
    let config = state.getObject("config") || { color: "#ff0000", speed: 1.0 };
    if (enabled === "true") {
        api.findById("sun").setAttribute("fill", config.color);
    }
}
```

### WidgetState API

| Method | Description |
|--------|-------------|
| `get(key)` | Retrieves a string value. Returns `""` if not found. |
| `set(key, value)` | Sets a persistent string value. |
| `getObject(key)` | Retrieves a JSON-deserialized object. Returns `null` if not found. |
| `setObject(key, obj)`| Serializes and stores an object as JSON. |
| `clear(key)` | Removes a key from the persistent state. |

For full typings, see [examples/widget.d.ts](examples/widget.d.ts).

## Interaction

- **Move**: Left-click and drag anywhere on the widget to move it.
- **Resize**: Hover over the bottom-right corner to reveal the resize handle. Left-click and drag the handle to resize the window.

## Packaging & Build System

The project includes a robust packaging environment based on Podman/Docker.

### Local Build (Binary + RPM + Flatpak)

1. **Build the Toolchain Image**:
   ```bash
   podman build -t waywidget-toolchain .
   ```

2. **Run the Packaging Script**:
   ```bash
   podman run --rm \
       --security-opt label=disable \
       --security-opt seccomp=unconfined \
       -v .:/build:Z \
       waywidget-toolchain
   ```
   Artifacts will be available in the `./dest` directory.

### Continuous Integration

Every push to the `main` branch on GitHub triggers an automated build. Artifacts (Binary, RPM, Flatpak) are automatically generated and attached to the GitHub Action run.

## Project Information

- **URL**: https://github.com/maxfridbe/WayWidget
- **Author**: Max Fridbe <maxfridbe@gmail.com>
- **License**: MIT

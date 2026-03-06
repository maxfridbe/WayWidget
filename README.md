# WayWidgets System

A lightweight, high-performance Wayland widget system that renders SVG templates using Cairo and provides dynamic logic via an embedded JavaScript engine (Boa).

![Sunrise Example](sunrise.png)

## Features

- **SVG Rendering**: Uses `librsvg` and `cairo` for crisp, vector-based visuals.
- **JS Scripting**: Dynamic updates via a fluent JavaScript API.
- **Interactive**: Built-in support for moving and resizing widgets via the Wayland protocol.
- **Generic**: Run any widget by providing an SVG and a JS script.
- **Multi-format Packaging**: Automated builds for Release Binaries, RPMs, and Flatpaks.

## Simple Widget Creation

Creating a widget is as simple as:
1.  **Providing an SVG**: Design your widget in any vector tool (Inkscape, Illustrator, etc.).
2.  **Writing a JS function**: Add dynamic logic via the `update()` function.
3.  **Running it**: Point the engine at your files. No compilation required.

```bash
# Display a static SVG
waywidget --svg my-widget.svg

# Add interactivity and logic
waywidget --svg my-widget.svg --script logic.js
```

## CLI Usage

| Parameter | Shorthand | Description | Default |
|-----------|-----------|-------------|---------|
| `--svg` | `-s` | Path to the SVG template file (Required) | - |
| `--script` | `-j` | Path to the JavaScript logic file | - |
| `--width` | - | Initial window width | `200` |
| `--height` | - | Initial window height | `200` |

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

The system looks for a global `update(api, timestamp, click, state, request)` function.

- `api`: The `WidgetAPI` instance for finding elements and manipulating their attributes.
- `timestamp`: The current time in milliseconds (useful for animations).
- `click`: An object `{ x: number, y: number }` representing normalized coordinates (0.0 to 1.0) of the last click, or `null` if no click occurred in the last frame.
- `state`: A persistent `WidgetState` store that survives between `update` calls.
- `request`: A `RefreshRequest` instance used to schedule the next update.

### Example: Interactive Sunrise (`widget.js`)

```javascript
function update(api, timestamp, click, state, request) {
    let enabled = state.get("enabled") || "true";

    if (click) {
        enabled = (enabled === "true") ? "false" : "true";
        state.set("enabled", enabled);
        console.log("Animation enabled:", enabled);
    }

    if (enabled === "true") {
        request.refreshInMS(33); // Request next update in 33ms (approx 30fps)
    }

    // Rich state saving with JSON
    let config = state.getObject("config") || { color: "#ff0000", speed: 1.0 };
    if (enabled === "true") {
        api.findById("sun").setAttribute("fill", config.color);
    }
}
```

### RefreshRequest API

| Method | Description |
|--------|-------------|
| `refreshInMS(ms)` | Requests the next `update()` call in `ms` milliseconds. Clamped to a minimum of `33ms` by the engine. |

### WidgetState API

| Method | Description |
|--------|-------------|
| `get(key)` | Retrieves a string value. Returns `""` if not found. |
| `set(key, value)` | Sets a persistent string value. |
| `getObject(key)` | Retrieves a JSON-deserialized object. Returns `null` if not found. |
| `setObject(key, obj)`| Serializes and stores an object as JSON. |
| `clear(key)` | Removes a key from the persistent state. |

### ElementHandle API

| Method | Description |
|--------|-------------|
| `setRotation(angle, cx?, cy?)` | Rotates the element around an optional center point. |
| `setTranslation(x, y)` | Moves the element by (x, y). |
| `setScale(factor)` | Scales the element (1.0 is default). |
| `setText(text)` | Sets the text content of the element. |
| `setAttribute(name, val)` | Sets a raw SVG attribute (e.g. "fill", "r"). |
| `setVisible(boolean)` | Toggles the `display: none` attribute. |
| `setOpacity(0.0-1.0)` | Sets the element's opacity. |
| `addClass(className)` | Adds a CSS class to the element. |
| `removeClass(className)` | Removes a CSS class from the element. |
| `appendElement(tag, attrs)` | Dynamically creates and appends a child SVG element. |
| `clearChildren()` | Removes all child nodes. |
| `remove()` | Removes the element from the SVG tree. |

For full typings, see [examples/widget.d.ts](examples/widget.d.ts).

## Interaction

- **Move**: Left-click and drag anywhere on the widget to move it.
- **Resize**: Hover over the bottom-right corner to reveal the resize handle. Left-click and drag the handle to resize the window.

## Window Manager Configuration

For tiling window managers, you should configure `waywidget` to always open as a floating window.

### Niri
Add to `~/.config/niri/config.kdl`:
```kdl
window-rule {
    match app-id="waywidget"
    open-floating true
}
```

### Sway
Add to `~/.config/sway/config`:
```sway
for_window [app_id="waywidget"] floating enable, border none
```

### Hyprland
Add to `~/.config/hypr/hyprland.conf`:
```hypr
windowrulev2 = float, class:(waywidget)
windowrulev2 = pin, class:(waywidget)
```

## Development & Testing

The system includes a robust suite of unit and integration tests to ensure reliable SVG manipulation and JavaScript integration.

To run the tests:
```bash
cd waywidget
cargo test
```

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

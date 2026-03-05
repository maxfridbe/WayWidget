/**
 * WayWidget Scripting Typings
 */

/**
 * Handle to a specific SVG element for manipulation.
 */
interface ElementHandle {
    /**
     * Sets the rotation of the element.
     * @param angle Rotation in degrees.
     * @param cx Optional center X coordinate (default 50).
     * @param cy Optional center Y coordinate (default 50).
     */
    setRotation(angle: number, cx?: number, cy?: number): ElementHandle;

    /**
     * Sets the inner text content of the element.
     * @param text The text to display.
     */
    setText(text: string): ElementHandle;

    /**
     * Sets a generic SVG attribute.
     * @param name Attribute name (e.g. "fill", "stroke-width").
     * @param value Attribute value.
     */
    setAttribute(name: string, value: string): ElementHandle;
}

/**
 * Global API passed to the update() function.
 */
interface WidgetAPI {
    /**
     * Finds an element by its ID in the SVG template.
     * @param id The ID attribute of the element.
     */
    findById(id: string): ElementHandle;
}

/**
 * Persistent state store for the widget.
 */
interface WidgetState {
    /**
     * Sets a persistent string value.
     * @param key The key to store.
     * @param value The value to store.
     */
    set(key: string, value: string): void;

    /**
     * Retrieves a persistent string value.
     * @param key The key to retrieve.
     * @returns The stored value, or an empty string if not found.
     */
    get(key: string): string;

    /**
     * Clears a persistent value from the state.
     * @param key The key to clear.
     */
    clear(key: string): void;

    /**
     * Sets a persistent object, serialized as JSON.
     * @param key The key to store.
     * @param value The object to store.
     */
    setObject(key: string, value: any): void;

    /**
     * Retrieves a persistent object, deserialized from JSON.
     * @param key The key to retrieve.
     * @returns The stored object, or null if not found.
     */
    getObject(key: string): any;
}

/**
 * The main entry point for widget logic.
 * This function is called on every update cycle.
 * @param api The WidgetAPI instance for manipulation.
 * @param timestamp Current time in milliseconds.
 * @param click Normalized coordinates of the last click (0-1), or undefined.
 * @param state Persistent state object for the widget.
 */
declare function update(api: WidgetAPI, timestamp: number, click?: { x: number, y: number }, state?: WidgetState): void;

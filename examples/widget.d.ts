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
     * Sets the translation of the element.
     * @param x Translation on the X axis.
     * @param y Translation on the Y axis.
     */
    setTranslation(x: number, y: number): ElementHandle;

    /**
     * Sets the scale of the element.
     * @param factor Scale factor (1.0 is default).
     */
    setScale(factor: number): ElementHandle;

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

    /**
     * Sets whether the element is visible.
     * @param visible True to show, false to hide.
     */
    setVisible(visible: boolean): ElementHandle;

    /**
     * Sets the opacity of the element.
     * @param opacity Opacity from 0.0 to 1.0.
     */
    setOpacity(opacity: number): ElementHandle;

    /**
     * Adds a CSS class to the element.
     * @param className The class name to add.
     */
    addClass(className: string): ElementHandle;

    /**
     * Removes a CSS class from the element.
     * @param className The class name to remove.
     */
    removeClass(className: string): ElementHandle;
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

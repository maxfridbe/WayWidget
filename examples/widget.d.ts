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
 * The main entry point for widget logic.
 * This function is called on every update cycle.
 * @param api The WidgetAPI instance for manipulation.
 */
declare function update(api: WidgetAPI): void;

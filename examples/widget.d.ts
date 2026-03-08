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

    /**
     * Appends a new child element to this element.
     * @param tag The SVG tag name (e.g. "circle", "rect").
     * @param attributes Object containing SVG attributes.
     */
    appendElement(tag: string, attributes: Record<string, string>): ElementHandle;

    /**
     * Removes all child nodes from this element.
     */
    clearChildren(): ElementHandle;

    /**
     * Removes this element from the SVG tree.
     */
    remove(): void;
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

    /**
     * Sets a truly global persistent string value across all widgets.
     * Saved to widgets_states.yml.
     * @param key The key to store.
     * @param value The value to store.
     */
    setGlobalPersistence(key: string, value: string): void;

    /**
     * Retrieves a truly global persistent string value.
     * @param key The key to retrieve.
     * @returns The stored value, or an empty string if not found.
     */
    getGlobalPersistence(key: string): string;
}

interface HttpResponse {
    status: number;
    body: string;
    error?: string;
}

interface WidgetResponse {
    /**
     * Normalized coordinates of the last click (0-1), or undefined.
     */
    click?: { x: number, y: number };
    
    /**
     * Array of strings representing keys pressed since last update.
     * Prefixed with '+' for press and '-' for release.
     */
    keyboard?: string[];

    /**
     * Map of URLs to their asynchronous HTTP responses.
     */
    http?: Record<string, HttpResponse>;

    /**
     * Map of command strings to their asynchronous CLI responses.
     */
    cli?: Record<string, { output: string, error?: string }>;
}

/**
 * Interface to request the next refresh cycle.
 */
interface RefreshRequest {
    /**
     * Requests a refresh in X milliseconds.
     * Clamped to a minimum of 33ms by the engine.
     * @param ms Delay in milliseconds.
     */
    refreshInMS(ms: number): void;

    /**
     * Enables keyboard event capture for the next frame.
     * Keys will be passed to the next update() call in the 'response.keyboard' parameter.
     */
    globalKeyboardEvents(): void;

    /**
     * Enables keyboard event capture for the next frame.
     * (Alias for globalKeyboardEvents)
     */
    localKeyboardEvents(): void;

    /**
     * Enables mouse click capture for the next frame.
     * If not called, the 'response.click' parameter in the next update() will be undefined.
     */
    localClickEvents(): void;

    /**
     * Triggers an asynchronous CLI command execution.
     * Results appear in response.cli[command] in a later update() call.
     * Must resolve within 10 seconds.
     * @param command The command to execute (e.g. "ip address").
     */
    CliInvoke(command: string): void;

    /**
     * Triggers an asynchronous JSON GET request.
     * Results appear in response.http[url] in a later update() call.
     * @param url The target URL.
     * @param headers Optional request headers.
     */
    jsonHttpGet(url: string, headers?: Record<string, string>): void;

    /**
     * Triggers an asynchronous JSON POST request.
     * @param url The target URL.
     * @param headers Optional request headers.
     * @param body The string body to send.
     */
    jsonHttpPost(url: string, headers?: Record<string, string>, body?: string): void;

    /**
     * Sends a message to another widget by name.
     * Works similarly to `waywidget message --name <name> --message <message>`.
     * @param name The name of the target widget.
     * @param message The message to send.
     */
    sendMessage(name: string, message: string): void;
}

/**
 * The main entry point for widget logic.
 * This function is called on every update cycle.
 * @param api The WidgetAPI instance for manipulation.
 * @param timestamp Current time in milliseconds.
 * @param response Object containing current frame events (click, keyboard, http, cli).
 * @param state Persistent state object for the widget.
 * @param request Interface to request the next refresh cycle.
 */
declare function update(api: WidgetAPI, timestamp: number, response: WidgetResponse, state: WidgetState, request: RefreshRequest): void;

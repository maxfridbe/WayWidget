function update(api, timestamp, response, state, request) {
    // Frequency for 0.25s interval (back and forth in 0.5s)
    // 0.25s = 250ms
    const freq = Math.PI / 250;
    const cycle = Math.sin(timestamp * freq);

    // 1. Bounce (0 to 5px)
    api.findById("fox").setTranslation(0, 2.5 + 2.5 * cycle);

    // 2. Wag (12deg to -12deg)
    api.findById("tail").setRotation(12 * cycle, 65, 78);

    // 3. Bob (-2deg to 4deg)
    api.findById("head").setRotation(1 + 3 * cycle, 125, 75);

    // 4. Running Legs (35deg to -35deg, alternating)
    const legAngle = 35 * cycle;
    api.findById("leg-hl-b").setRotation(legAngle, 75, 82);
    api.findById("leg-hl-f").setRotation(-legAngle, 75, 82);
    api.findById("leg-fl-b").setRotation(-legAngle, 120, 82);
    api.findById("leg-fl-f").setRotation(legAngle, 120, 82);

    // 5. Shadow Pulse (scale 1.0 to 0.85, opacity 0.15 to 0.05)
    const shadowScale = 0.925 + 0.075 * cycle;
    api.findById("shadow").setScale(shadowScale);
    api.findById("shadow").setAttribute("opacity", (0.10 + 0.05 * cycle).toString());

    // Request 60 FPS update
    request.refreshInMS(16);
}

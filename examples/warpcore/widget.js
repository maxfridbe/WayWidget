function update(api, timestamp, response, state, request) {
    if (request) {
        request.localKeyboardEvents();
        request.refreshInMS(33); // High frequency for smooth pulsing
    }

    // 1. Handle Speed via Keyboard (+/-)
    let speed = parseFloat(state.get("warp_speed") || "1.8");
    const minSpeed = 0.2;
    const maxSpeed = 5.0;
    const step = 0.2;

    if (response.keyboard && response.keyboard.length > 0) {
        response.keyboard.forEach(k => {
            if (k.startsWith('+')) {
                let name = k.substring(1);
                if (name.startsWith("XK_")) name = name.substring(3);
                
                if (name === "equal" || name === "plus") {
                    speed = Math.max(minSpeed, speed - step);
                } else if (name === "minus" || name === "underscore") {
                    speed = Math.min(maxSpeed, speed + step);
                }
            }
        });
        state.set("warp_speed", speed.toString());
        console.log("Warp intermix speed:", speed.toFixed(1), "s");
    }

    // 2. Animation Logic
    const durationMS = speed * 1000;
    const progress = (timestamp % durationMS) / durationMS; // 0.0 to 1.0

    // Pulsing helper
    function getPulse(progress, offset) {
        // Shift progress by offset and wrap
        const p = (progress + offset) % 1.0;
        // Triangle wave: 0 -> 1 -> 0
        return p < 0.5 ? p * 2 : 2 - p * 2;
    }

    // 3. Update Segments
    // Top Segments (Pulse outside-in: top to center)
    for (let i = 0; i < 6; i++) {
        // Reverse i so top (0) has earlier pulse than center (5)
        const pulse = getPulse(progress, (5 - i) * 0.12);
        const opacity = 0.3 + (pulse * 0.7);
        api.findById(`seg-t-${i}`).setOpacity(opacity);
    }

    // Bottom Segments (Pulse outside-in: bottom to center)
    for (let i = 0; i < 6; i++) {
        // i=5 is bottom, i=0 is near center. 
        // Bottom (5) should pulse first.
        const pulse = getPulse(progress, i * 0.12);
        const opacity = 0.3 + (pulse * 0.7);
        const id = i === 4 ? "id-seg-b-4" : `seg-b-${i}`;
        api.findById(id).setOpacity(opacity);
    }

    // 4. Update Intermix Chamber
    // Chamber itself stays solid
    api.findById("chamber").setOpacity(1.0);
    
    // Only the very center circle pulsates
    const corePulse = getPulse(progress, 0.7);
    const coreOpacity = 0.4 + (corePulse * 0.6);
    api.findById("pulsating-core").setOpacity(coreOpacity);
}

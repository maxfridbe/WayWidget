const LOOP_DURATION = 20000; 

function interpolateColor(color1, color2, factor) {
    const r1 = parseInt(color1.substring(1, 3), 16);
    const g1 = parseInt(color1.substring(3, 5), 16);
    const b1 = parseInt(color1.substring(5, 7), 16);
    const r2 = parseInt(color2.substring(1, 3), 16);
    const g2 = parseInt(color2.substring(3, 5), 16);
    const b2 = parseInt(color2.substring(5, 7), 16);
    const r = Math.round(r1 + factor * (r2 - r1));
    const g = Math.round(g1 + factor * (g2 - g1));
    const b = Math.round(b1 + factor * (b2 - b1));
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
}

function update(api, timestamp, click, state, request) {
    // 1. Handle Click Interactivity (Toggle Pause)
    let enabled = state.get("enabled");
    if (enabled === "") { // Initial state
        enabled = "true";
        state.set("enabled", "true");
    }

    if (click) {
        enabled = (enabled === "true") ? "false" : "true";
        state.set("enabled", enabled);
        console.log("Sunrise animation enabled:", enabled);
    }

    if (request && enabled === "true") {
        request.refreshInMS(33); // Smooth 30fps animation
    }

    // 2. Logic: If paused, use a stored 'pause_time' or just freeze the frame
    let effectiveTime = timestamp;
    if (enabled === "false") {
        let pauseTime = state.get("pause_time");
        if (pauseTime === "") {
            state.set("pause_time", timestamp.toString());
            effectiveTime = timestamp;
        } else {
            effectiveTime = parseFloat(pauseTime);
        }
    } else {
        // When unpausing, we might want to offset the time to prevent jumps,
        // but for now, let's keep it simple and just freeze.
        state.clear("pause_time"); 
    }

    const elapsed = effectiveTime % LOOP_DURATION;
    const progress = elapsed / LOOP_DURATION; // 0 to 1

    // 1. Sun Trajectory
    const angle = (progress * (Math.PI * 1.25)) - 0.1; 
    const centerX = 400;
    const centerY = 450;
    const radius = 350;

    const sunX = centerX - Math.cos(angle) * radius * 1.5;
    const sunY = centerY - Math.sin(angle) * radius;
    
    api.findById('sun').setAttribute('cx', sunX.toString()).setAttribute('cy', sunY.toString());
    api.findById('glow').setAttribute('cx', sunX.toString()).setAttribute('cy', sunY.toString());

    // 2. Environment Colors
    let skyColor, sunColor, landColor, nightAlpha;
    
    if (progress < 0.2) { // Dawn
        const p = progress / 0.2;
        skyColor = interpolateColor('#020617', '#fdba74', p);
        sunColor = interpolateColor('#ef4444', '#fbbf24', p);
        landColor = interpolateColor('#022c22', '#166534', p);
        nightAlpha = 1 - p;
    } else if (progress < 0.5) { // Day
        const p = (progress - 0.2) / 0.3;
        skyColor = interpolateColor('#fdba74', '#7dd3fc', p);
        sunColor = '#fef08a';
        landColor = '#15803d';
        nightAlpha = 0;
    } else if (progress < 0.8) { // Sunset
        const p = (progress - 0.5) / 0.3;
        skyColor = interpolateColor('#7dd3fc', '#1e1b4b', p);
        sunColor = interpolateColor('#fef08a', '#b91c1c', p);
        landColor = interpolateColor('#15803d', '#022c22', p);
        nightAlpha = p;
    } else { // Night
        skyColor = '#020617';
        sunColor = '#7f1d1d';
        landColor = '#020617';
        nightAlpha = 1;
    }

    api.findById('sky').setAttribute('fill', skyColor);
    api.findById('sun').setAttribute('fill', sunColor);
    api.findById('land').setAttribute('fill', landColor);
    api.findById('stars').setAttribute('opacity', nightAlpha.toString());
    api.findById('glow').setAttribute('opacity', (1 - nightAlpha * 0.8).toString());

    // 3. Cloud Movement
    const clouds = [
        { id: 'cloud1', x: 150, speed: 1.2 },
        { id: 'cloud2', x: 600, speed: 0.8 },
        { id: 'cloud3', x: 300, speed: 1.0 }
    ];

    clouds.forEach(c => {
        let currentX = (c.x + (effectiveTime / 50) * c.speed) % 1000;
        if (currentX > 900) currentX -= 1000; 
        api.findById(c.id).setAttribute('cx', (currentX - 50).toString());
        api.findById(c.id).setAttribute('opacity', ((1 - nightAlpha) * 0.5).toString());
    });

    // Update timer
    const seconds = Math.floor(elapsed / 100);
    api.findById('timer').setText(`Cycle: ${seconds}%${enabled === "false" ? " (PAUSED)" : ""}`);
}

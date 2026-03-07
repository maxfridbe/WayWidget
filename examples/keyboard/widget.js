function update(api, timestamp, click, keys, state, request) {
    if (request) {
        request.globalKeyboardEvents();
    }

    let activeKeys = state.getObject("active_keys") || {};
    let shiftHeld = state.get("shift_held") === "true";
    let changed = false;

    // Special mappings for keysym names
    const keyMap = {
        "grave": "grave",
        "asciitilde": "grave",
        "minus": "minus",
        "underscore": "minus",
        "equal": "equal",
        "plus": "equal",
        "Shift_L": "Shift_L",
        "Shift_R": "Shift_R",
        "Escape": "Escape"
    };

    if (keys && keys.length > 0) {
        keys.forEach(k => {
            const isPress = k.startsWith('+');
            let name = k.substring(1);
            
            if (name === "Shift_L" || name === "Shift_R") {
                shiftHeld = isPress;
                state.set("shift_held", shiftHeld ? "true" : "false");
                if (shiftHeld) {
                    api.findById("svg-root").addClass("shift-active");
                } else {
                    api.findById("svg-root").removeClass("shift-active");
                }
                changed = true;
            }

            if (isPress) {
                if (name.startsWith("XK_")) name = name.substring(3);
                const mappedName = keyMap[name] || name;
                
                activeKeys[mappedName] = timestamp + 100;
                const el = api.findById(`key-${mappedName}`) || 
                           api.findById(`key-${mappedName.toLowerCase()}`);
                
                if (el) {
                    el.addClass('active');
                    changed = true;
                }
            }
        });
    }

    for (let name in activeKeys) {
        if (timestamp >= activeKeys[name]) {
            const el = api.findById(`key-${name}`) || api.findById(`key-${name.toLowerCase()}`);
            if (el) {
                el.removeClass('active');
                changed = true;
            }
            delete activeKeys[name];
        }
    }

    state.setObject("active_keys", activeKeys);

    if (Object.keys(activeKeys).length > 0 || changed) {
        request.refreshInMS(33);
    }
}

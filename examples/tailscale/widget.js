const POLL_INTERVAL = 10000; // Poll every 10 seconds
const TS_CMD = "distrobox-host-exec tailscale status --json";

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        
        // Initial request or scheduled refresh
        let lastUpdate = parseInt(state.get("last_update") || "0");
        if (timestamp - lastUpdate > POLL_INTERVAL) {
            request.CliInvoke(TS_CMD);
            state.set("last_update", timestamp.toString());
        }

        // Fast refresh if we just clicked
        if (response.click) {
            request.CliInvoke(TS_CMD);
            state.set("last_update", timestamp.toString());
            request.refreshInMS(100);
        } else {
            request.refreshInMS(1000); // Check for CLI response every second
        }
    }

    // Process CLI output
    if (response.cli && response.cli[TS_CMD]) {
        let cliResult = response.cli[TS_CMD];
        if (!cliResult.error) {
            try {
                let status = JSON.parse(cliResult.output);
                updateUI(api, status);
            } catch (e) {
                console.log("Error parsing Tailscale JSON: " + e);
            }
        } else {
            api.findById("hostname").setText("Error");
            api.findById("status-dot").setAttribute("fill", "#ff4444");
        }
    }
}

function updateUI(api, status) {
    // Hostname
    let hostname = status.Self.HostName || "Unknown";
    api.findById("hostname").setText(hostname);

    // IP
    let ip = (status.Self.TailscaleIPs && status.Self.TailscaleIPs[0]) || "--.---.---.--";
    api.findById("ip").setText(ip);

    // Backend State / Color
    let stateColor = "#555555";
    if (status.BackendState === "Running") {
        stateColor = "#44ff44"; // Connected
    } else if (status.BackendState === "NeedsLogin" || status.BackendState === "Stopped") {
        stateColor = "#ffaa00"; // Warn
    }
    api.findById("status-dot").setAttribute("fill", stateColor);

    // Peers
    let onlinePeers = 0;
    if (status.Peer) {
        for (let p in status.Peer) {
            if (status.Peer[p].Online) {
                onlinePeers++;
            }
        }
    }
    api.findById("peers-count").setText(onlinePeers.toString());
}

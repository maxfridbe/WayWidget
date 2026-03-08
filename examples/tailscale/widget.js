const POLL_INTERVAL = 30000; // Poll every 30 seconds
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

        // Handle clicks for copying and toast
        if (response.click) {
            let elementId = response.click.id;
            if (elementId && elementId.startsWith("peer-row-")) {
                let ip = elementId.replace("peer-row-", "");
                request.CliInvoke("echo -n " + ip + " | distrobox-host-exec wl-copy");
                showToast(api, state, timestamp);
            }
        }

        // Auto-hide toast after 2 seconds
        let toastShownAt = parseInt(state.get("toast_shown_at") || "0");
        if (toastShownAt > 0 && timestamp - toastShownAt > 2000) {
            api.findById("toast-bg").setOpacity(0);
            api.findById("toast-text").setOpacity(0);
            state.set("toast_shown_at", "0");
        }

        request.refreshInMS(1000); 
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
        }
    }
}

function showToast(api, state, timestamp) {
    api.findById("toast-bg").setOpacity(0.9);
    api.findById("toast-text").setOpacity(1);
    state.set("toast_shown_at", timestamp.toString());
}

function updateUI(api, status) {
    // Self Info
    let hostname = status.Self.HostName || "Unknown";
    api.findById("hostname").setText(hostname);

    let ip = (status.Self.TailscaleIPs && status.Self.TailscaleIPs[0]) || "--.---.---.--";
    api.findById("self-ip").setText(ip);

    let stateColor = status.BackendState === "Running" ? "#44ff44" : "#ffaa00";
    api.findById("status-dot").setAttribute("fill", stateColor);

    // Peer List
    let container = api.findById("peer-list");
    container.clearChildren();

    let peersToShow = [];
    if (status.Peer) {
        for (let p in status.Peer) {
            let peer = status.Peer[p];
            if (peer.Online) {
                peersToShow.push(peer);
            }
        }
    }

    console.log("Tailscale Peers found: " + (status.Peer ? Object.keys(status.Peer).length : 0));
    console.log("Online Peers: " + peersToShow.length);

    // If no one is online, show everyone for debugging purposes
    if (peersToShow.length === 0 && status.Peer) {
        for (let p in status.Peer) {
            peersToShow.push(status.Peer[p]);
        }
    }

    // Sort peers by hostname
    peersToShow.sort((a, b) => a.HostName.localeCompare(b.HostName));
    api.findById("peers-online").setText(peersToShow.length + " Total/Online");

    // Render peer rows
    let yOffset = 0;
    const ROW_HEIGHT = 35;
    const MAX_VISIBLE = 8; 

    peersToShow.slice(0, MAX_VISIBLE).forEach((peer, index) => {
        let peerIP = peer.TailscaleIPs[0];
        
        // Background rectangle for interaction
        container.appendElement("rect", {
            id: "peer-row-" + peerIP,
            x: "-5",
            y: yOffset.toString(),
            width: "300",
            height: (ROW_HEIGHT - 5).toString(),
            rx: "4",
            fill: index % 2 === 0 ? "#1a1a1a" : "#151515",
            opacity: "0.8"
        });

        // Peer hostname
        container.appendElement("text", {
            x: "10",
            y: (yOffset + 14).toString(),
            "font-family": "sans-serif",
            "font-size": "11",
            "font-weight": "bold",
            fill: "white",
            style: "pointer-events: none;"
        }).setText(peer.HostName);

        // Peer IP
        container.appendElement("text", {
            x: "10",
            y: (yOffset + 26).toString(),
            "font-family": "monospace",
            "font-size": "9",
            fill: "#888",
            style: "pointer-events: none;"
        }).setText(peerIP);

        // Online dot
        container.appendElement("circle", {
            cx: "285",
            cy: (yOffset + 15).toString(),
            r: "3",
            fill: "#44ff44",
            style: "pointer-events: none;"
        });

        yOffset += ROW_HEIGHT;
    });

    if (onlinePeers.length === 0) {
        container.appendElement("text", {
            x: "145",
            y: "50",
            "font-family": "sans-serif",
            "font-size": "12",
            fill: "#666",
            "text-anchor": "middle"
        }).setText("No online peers found.");
    } else if (onlinePeers.length > MAX_VISIBLE) {
        container.appendElement("text", {
            x: "10",
            y: (yOffset + 10).toString(),
            "font-family": "sans-serif",
            "font-size": "10",
            fill: "#444"
        }).setText("+ " + (onlinePeers.length - MAX_VISIBLE) + " more peers...");
    }
}

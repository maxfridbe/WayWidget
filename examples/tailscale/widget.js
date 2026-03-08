const POLL_INTERVAL = 30000;
const TS_CMD = "distrobox-host-exec tailscale status --json";

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        
        let lastUpdate = parseInt(state.get("last_update") || "0");
        if (timestamp - lastUpdate > POLL_INTERVAL || lastUpdate === 0) {
            request.CliInvoke(TS_CMD);
            state.set("last_update", timestamp.toString());
        }

        if (response.click) {
            let id = response.click.id;
            if (id && id.startsWith("peer-card-")) {
                let ip = id.replace("peer-card-", "");
                request.CliInvoke("echo -n " + ip + " | distrobox-host-exec wl-copy");
                api.findById("toast").setOpacity(1);
                state.set("toast_at", timestamp.toString());
            }
        }

        let toastAt = parseInt(state.get("toast_at") || "0");
        if (toastAt > 0 && timestamp - toastAt > 2000) {
            api.findById("toast").setOpacity(0);
            state.set("toast_at", "0");
        }

        request.refreshInMS(1000);
    }

    if (response.cli && response.cli[TS_CMD]) {
        let res = response.cli[TS_CMD];
        if (!res.error) {
            try {
                let status = JSON.parse(res.output);
                renderPeers(api, status);
            } catch(e) { console.log("JSON Parse Error: " + e); }
        }
    }
}

function renderPeers(api, status) {
    // Header
    let isRunning = status.BackendState === "Running";
    api.findById("status-dot").setAttribute("class", isRunning ? "status-online" : "status-offline");
    api.findById("status-text").setText(isRunning ? "Connected" : status.BackendState);

    let list = api.findById("peer-list");
    list.clearChildren();

    let peers = [];
    if (status.Peer) {
        for (let p in status.Peer) {
            let peer = status.Peer[p];
            if (peer.Online) peers.push(peer);
        }
    }
    
    // Add "This device" to the top
    let self = status.Self;
    if (self) {
        peers.unshift({
            HostName: self.HostName + " (this device)",
            TailscaleIPs: self.TailscaleIPs,
            OS: self.OS,
            Online: true,
            IsSelf: true
        });
    }

    let y = 0;
    const ROW_H = 52;
    
    peers.slice(0, 5).forEach((peer, i) => {
        let ip = peer.TailscaleIPs[0];
        let cardId = "peer-card-" + ip;

        // Group for the row
        let row = list.appendElement("g", { transform: "translate(0, " + y + ")" });
        
        // Background card
        row.appendElement("rect", {
            id: cardId,
            width: "318",
            height: "44",
            class: "card",
            rx: "8"
        });

        // Status dot
        row.appendElement("circle", {
            cx: "16",
            cy: "22",
            r: "4",
            class: "status-online"
        });

        // Hostname
        row.appendElement("text", {
            x: "28",
            y: "20",
            class: "text-primary",
            "font-size": "14",
            "font-weight": "500",
            style: "pointer-events: none;"
        }).setText(peer.HostName);

        // OS / Meta
        let meta = (peer.OS || "Unknown") + (peer.ExitNode ? " • Exit Node" : "");
        row.appendElement("text", {
            x: "28",
            y: "34",
            class: "text-secondary",
            style: "pointer-events: none;"
        }).setText(meta);

        // IP
        row.appendElement("text", {
            x: "306",
            y: "26",
            "text-anchor": "end",
            class: "text-ip",
            style: "pointer-events: none;"
        }).setText(ip);

        y += ROW_H;
    });

    if (peers.length === 0) {
        list.appendElement("text", { x: "159", y: "50", class: "text-secondary", "text-anchor": "middle" })
            .setText("No online peers found");
    }
}

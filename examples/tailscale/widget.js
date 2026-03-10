const POLL_INTERVAL = 30000;
const TS_CMD = "tailscale status --json";
let LAST_EXIT_CMD = "";

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        
        let lastUpdate = parseInt(state.get("last_update") || "0");
        if (timestamp - lastUpdate > POLL_INTERVAL || lastUpdate === 0) {
            console.log("Invoking Tailscale status command...");
            request.CliInvoke(TS_CMD);
            state.set("last_update", timestamp.toString());
        }

        if (response.click) {
            let id = response.click.id;
            console.log("Clicked ID: " + id);
            if (id && id.startsWith("exit-btn-")) {
                let ip = id.replace("exit-btn-", "");
                let cmd = "tailscale up --exit-node=" + ip;
                console.log("Setting exit node with cmd: " + cmd);
                LAST_EXIT_CMD = cmd;
                request.CliInvoke(cmd);
                showToast(api, state, timestamp, "Setting exit node...");
            } else if (id && id.startsWith("deactivate-btn")) {
                let cmd = "tailscale up --exit-node=";
                console.log("Deactivating exit node with cmd: " + cmd);
                LAST_EXIT_CMD = cmd;
                request.CliInvoke(cmd);
                showToast(api, state, timestamp, "Deactivating exit node...");
            } else if (id && id.startsWith("peer-card-")) {
                let ip = id.replace("peer-card-", "");
                request.CliInvoke("echo -n " + ip + " | wl-copy");
                showToast(api, state, timestamp, "IP Copied: " + ip);
            }
        }

        // Check for CLI response from exit node command
        if (LAST_EXIT_CMD && response.cli && response.cli[LAST_EXIT_CMD]) {
            let res = response.cli[LAST_EXIT_CMD];
            console.log("Exit Node CLI Response: " + JSON.stringify(res));
            let msg = res.error ? "Error: " + res.error : "Exit node set!";
            showToast(api, state, timestamp, msg);
            LAST_EXIT_CMD = "";
            request.CliInvoke(TS_CMD); // Refresh status
        }

        let toastAt = parseInt(state.get("toast_at") || "0");
        if (toastAt > 0 && timestamp - toastAt > 3000) {
            api.findById("toast").setOpacity(0);
            state.set("toast_at", "0");
        }

        request.refreshInMS(1000);
    }

    if (response.cli) {
        if (response.cli[TS_CMD]) {
            let res = response.cli[TS_CMD];
            if (!res.error) {
                try {
                    let status = JSON.parse(res.output);
                    renderPeers(api, status);
                } catch(e) { 
                    console.log("JSON Parse Error: " + e);
                    console.log("Raw output was: " + res.output.substring(0, 100) + "...");
                }
            } else {
                console.log("CLI Error for status: " + res.error);
            }
        }
    }
}

function showToast(api, state, timestamp, message) {
    api.findById("toast-text-el").setText(message);
    api.findById("toast").setOpacity(1);
    state.set("toast_at", timestamp.toString());
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
            if (peer.Online || peer.ExitNodeOption) peers.push(peer);
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
            IsSelf: true,
            CurAddr: self.CurAddr,
            Relay: self.Relay
        });
    }

    let y = 0;
    const ROW_H = 52;
    
    peers.slice(0, 5).forEach((peer, i) => {
        let ip = peer.TailscaleIPs[0];
        let cardId = "peer-card-" + ip;
        let exitBtnId = "exit-btn-" + ip;

        // Create row group
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
            class: peer.Online ? "status-online" : "status-offline"
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
        let connType = peer.CurAddr ? "Direct" : (peer.Relay ? "Relay (" + peer.Relay + ")" : "Offline");
        if (peer.IsSelf) connType = "This device";
        let meta = (peer.OS || "Unknown") + " • " + connType + (peer.ExitNode ? " • Active Exit" : "");
        row.appendElement("text", {
            x: "28",
            y: "34",
            class: "text-secondary",
            style: "pointer-events: none;"
        }).setText(meta);

        // Exit Node Button
        if (peer.ExitNodeOption && !peer.IsSelf && !peer.ExitNode) {
            let btnG = row.appendElement("g", {});
            btnG.appendElement("rect", {
                id: exitBtnId,
                width: "60",
                height: "18",
                x: "245",
                y: "13",
                rx: "9",
                fill: "#34C759",
                style: "cursor: pointer;"
            });
            btnG.appendElement("text", {
                x: "275",
                y: "25",
                "font-size": "8",
                "font-weight": "bold",
                fill: "white",
                "text-anchor": "middle",
                style: "pointer-events: none;"
            }).setText("USE EXIT");
        } else if (peer.ExitNode) {
            // Active exit node - show red deactivate button
            let btnG = row.appendElement("g", {});
            btnG.appendElement("rect", {
                id: "deactivate-btn-" + i,
                width: "70",
                height: "18",
                x: "235",
                y: "13",
                rx: "9",
                fill: "#FF3B30",
                style: "cursor: pointer;"
            });
            btnG.appendElement("text", {
                x: "270",
                y: "25",
                "font-size": "8",
                "font-weight": "bold",
                fill: "white",
                "text-anchor": "middle",
                style: "pointer-events: none;"
            }).setText("DEACTIVATE");
        } else {
            // Just show IP
            row.appendElement("text", {
                x: "306",
                y: "26",
                "text-anchor": "end",
                class: "text-ip",
                style: "pointer-events: none;"
            }).setText(ip);
        }

        y += ROW_H;
    });

    if (peers.length === 0) {
        list.appendElement("text", { x: "159", y: "50", class: "text-secondary", "text-anchor": "middle" })
            .setText("No online peers found");
    }
}

const LOCAL_CMD = "ip address";
const PUBLIC_URL = "https://api.ipify.org?format=json";

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        request.refreshInMS(1000);
    }

    let isFetchingLocal = state.get("fetching_local") === "true";
    let isFetchingPublic = state.get("fetching_public") === "true";
    let lastFetched = parseInt(state.get("last_fetched") || "0");

    const now = Date.now();
    const shouldFetch = (now - lastFetched > 60000) || (response.click && !isFetchingLocal && !isFetchingPublic);

    // 1. Start fetching if needed
    if (shouldFetch) {
        console.log("Refreshing Network Data...");
        
        request.CliInvoke(LOCAL_CMD);
        state.set("fetching_local", "true");
        api.findById("local-ip").setText("Invoking...");

        request.jsonHttpGet(PUBLIC_URL);
        state.set("fetching_public", "true");
        api.findById("public-ip").setText("Requesting...");
    }

    // 2. Process Local IP (CLI)
    if (response.cli && response.cli[LOCAL_CMD]) {
        const res = response.cli[LOCAL_CMD];
        state.set("fetching_local", "false");
        
        if (res.error) {
            api.findById("local-ip").setText("CLI Error");
        } else {
            // Find first non-loopback inet address
            const lines = res.output.split('\n');
            let found = "No IP Found";
            for (let line of lines) {
                const match = line.match(/inet\s(\d+\.\d+\.\d+\.\d+)/);
                if (match && match[1] !== "127.0.0.1") {
                    found = match[1];
                    break;
                }
            }
            api.findById("local-ip").setText(found);
        }
    }

    // 3. Process Public IP (HTTP)
    if (response.http && response.http[PUBLIC_URL]) {
        const res = response.http[PUBLIC_URL];
        state.set("fetching_public", "false");
        
        if (res.status === 200) {
            try {
                const ip = JSON.parse(res.body).ip;
                api.findById("public-ip").setText(ip);
                
                const d = new Date();
                const timeStr = `${d.getHours()}:${d.getMinutes()}:${d.getSeconds()}`;
                api.findById("last-updated").setText(`Last updated: ${timeStr}`);
                state.set("last_fetched", now.toString());
            } catch (e) {
                api.findById("public-ip").setText("Parse Error");
            }
        } else {
            api.findById("public-ip").setText("HTTP Error");
        }
    }
}

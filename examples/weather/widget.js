const ZIP_API = "https://api.zippopotam.us/us/";
const NWS_POINTS_API = "https://api.weather.gov/points/";

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
    }

    let zipCode = state.getGlobalPersistence("weather_zip") || "63141";
    if (state.getGlobalPersistence("weather_zip") === "") {
        state.setGlobalPersistence("weather_zip", zipCode);
    }

    let step = state.get("fetch_step") || "init";
    if (request) {
        if (step === "init") {
            request.refreshInMS(10000);
        } else {
            request.refreshInMS(500); // Poll faster when waiting for HTTP
        }
    }

    let lastFetched = parseInt(state.get("last_fetched") || "0");
    const now = Date.now();

    const forecastUrlKey = "forecast_url_" + zipCode;
    const locNameKey = "location_name_" + zipCode;

    // 1. Trigger Fetch
    if (step === "init" && (now - lastFetched > 3600000 || response.click)) {
        let cachedForecastUrl = state.getGlobalPersistence(forecastUrlKey);
        
        if (cachedForecastUrl && !response.click) {
            // Jump straight to forecast fetch using cached URL for this zip
            request.jsonHttpGet(cachedForecastUrl);
            state.set("active_forecast_url", cachedForecastUrl);
            state.set("fetch_step", "nws_forecast");
            api.findById("status").setText("Refreshing...");
        } else {
            request.jsonHttpGet(ZIP_API + zipCode);
            state.set("fetch_step", "zippopotam");
            api.findById("status").setText("Locating zip...");
        }
    }

    // 2. Handle Zippopotam
    if (step === "zippopotam" && response.http && response.http[ZIP_API + zipCode]) {
        const res = response.http[ZIP_API + zipCode];
        if (res.status === 200) {
            const data = JSON.parse(res.body);
            const lat = data.places[0].latitude;
            const lon = data.places[0].longitude;
            const locName = data.places[0]["place name"] + ", " + data.places[0].state;
            
            state.setGlobalPersistence(locNameKey, locName);
            
            const pointsUrl = NWS_POINTS_API + lat + "," + lon;
            request.jsonHttpGet(pointsUrl);
            state.set("points_url", pointsUrl);
            state.set("fetch_step", "nws_points");
        } else {
            state.set("fetch_step", "init");
            api.findById("status").setText("Zip error");
        }
    }

    // 3. Handle NWS Points
    const pointsUrl = state.get("points_url");
    if (step === "nws_points" && response.http && response.http[pointsUrl]) {
        const res = response.http[pointsUrl];
        if (res.status === 200) {
            const forecastUrl = JSON.parse(res.body).properties.forecast;
            request.jsonHttpGet(forecastUrl);
            state.setGlobalPersistence(forecastUrlKey, forecastUrl);
            state.set("active_forecast_url", forecastUrl);
            state.set("fetch_step", "nws_forecast");
        } else {
            state.set("fetch_step", "init");
        }
    }

    // 4. Handle Forecast
    const activeUrl = state.get("active_forecast_url") || state.getGlobalPersistence(forecastUrlKey);
    if (step === "nws_forecast" && response.http && response.http[activeUrl]) {
        const res = response.http[activeUrl];
        if (res.status === 200) {
            const periods = JSON.parse(res.body).properties.periods;
            const daily = periods.filter(p => p.isDaytime).slice(0, 7);
            
            daily.forEach((day, i) => {
                const name = i === 0 ? "Today" : day.name.substring(0, 3);
                api.findById(`day-name-${i}`).setText(name);
                api.findById(`temp-${i}`).setText(`${day.temperature}°`);
                
                // Icon Mapping
                const desc = day.shortForecast.toLowerCase();
                let icon = "icon-clear";
                if (desc.includes("rain") || desc.includes("showers")) icon = "icon-rain";
                else if (desc.includes("cloud") || desc.includes("overcast")) icon = "icon-cloudy";
                else if (desc.includes("storm") || desc.includes("thunder")) icon = "icon-storm";
                else if (desc.includes("snow") || desc.includes("ice")) icon = "icon-snow";
                
                api.findById(`icon-${i}`).setAttribute("href", `#${icon}`);
            });

            const locName = state.getGlobalPersistence(locNameKey) || "Weather";
            api.findById("location-label").setText(locName + " (" + zipCode + ")");
            api.findById("status").setText("Updated " + new Date().toISOString().slice(11, 19));
            state.set("fetch_step", "init");
            state.set("last_fetched", now.toString());
        } else {
            state.set("fetch_step", "init");
        }
    }
}

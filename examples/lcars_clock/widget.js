function pad(n) {
    return n < 10 ? '0' + n : n;
}

function update(api, timestamp, click, state, request) {
    if (request) {
        request.refreshInMS(1000); // Update every second
    }
    if (click) {
        console.log("LCARS clicked at:", click.x, click.y);
    }
    const now = new Date();
    
    // Date: MM DD YY
    const m = pad(now.getMonth() + 1);
    const d = pad(now.getDate());
    const y = String(now.getFullYear()).slice(-2);
    
    // Day
    const days = ['SUNDAY', 'MONDAY', 'TUESDAY', 'WEDNESDAY', 'THURSDAY', 'FRIDAY', 'SATURDAY'];
    const day = days[now.getDay()];
    
    // Time
    let h = now.getHours();
    const ampm = h >= 12 ? 'PM' : 'AM';
    h = h % 12 || 12;
    const min = pad(now.getMinutes());
    const sec = pad(now.getSeconds());
    
    api.findById("date-display").setText(`${m} ${d} ${y}`);
    api.findById("day-display").setText(day);
    api.findById("time-display").setText(`${pad(h)}:${min}`);
    api.findById("ampm-display").setText(ampm);
    api.findById("sec-display").setText(sec);
}

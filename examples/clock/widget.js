function update(api, timestamp, click) {
    if (click) {
        console.log("Clock clicked at:", click.x, click.y);
    }
    const now = new Date();
    const hours = now.getHours() % 12;
    const minutes = now.getMinutes();
    const seconds = now.getSeconds();

    const hRot = (hours * 30) + (minutes * 0.5);
    const mRot = minutes * 6;
    const sRot = seconds * 6;

    api.findById("hour_hand").setRotation(hRot, 50, 50);
    api.findById("minute_hand").setRotation(mRot, 50, 50);
    api.findById("second_hand").setRotation(sRot, 50, 50);
}

const BUTTONS = [
    { id: "C", x: 15, y: 80, w: 50, h: 45, val: "C" },
    { id: "paren-open", x: 71, y: 80, w: 50, h: 45, val: "(" },
    { id: "paren-close", x: 127, y: 80, w: 50, h: 45, val: ")" },
    { id: "div", x: 183, y: 80, w: 50, h: 45, val: "/" },

    { id: "7", x: 15, y: 135, w: 50, h: 45, val: "7" },
    { id: "8", x: 71, y: 135, w: 50, h: 45, val: "8" },
    { id: "9", x: 127, y: 135, w: 50, h: 45, val: "9" },
    { id: "mul", x: 183, y: 135, w: 50, h: 45, val: "*" },

    { id: "4", x: 15, y: 190, w: 50, h: 45, val: "4" },
    { id: "5", x: 71, y: 190, w: 50, h: 45, val: "5" },
    { id: "6", x: 127, y: 190, w: 50, h: 45, val: "6" },
    { id: "sub", x: 183, y: 190, w: 50, h: 45, val: "-" },

    { id: "1", x: 15, y: 245, w: 50, h: 45, val: "1" },
    { id: "2", x: 71, y: 245, w: 50, h: 45, val: "2" },
    { id: "3", x: 127, y: 245, w: 50, h: 45, val: "3" },
    { id: "add", x: 183, y: 245, w: 50, h: 45, val: "+" },

    { id: "0", x: 15, y: 300, w: 50, h: 45, val: "0" },
    { id: "dot", x: 71, y: 300, w: 50, h: 45, val: "." },
    { id: "back", x: 127, y: 300, w: 50, h: 45, val: "back" },
    { id: "equal", x: 183, y: 300, w: 50, h: 45, val: "=" }
];

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        request.localKeyEvents();
    }

    let expression = state.get("expression") || "";
    let result = state.get("result") || "0";
    let lastInput = state.get("last_input") || "";

    function handleInput(input) {
        if (input === "C") {
            expression = "";
            result = "0";
        } else if (input === "back") {
            expression = expression.slice(0, -1);
        } else if (input === "=") {
            try {
                // Basic cleanup for eval-like behavior
                let cleanExpr = expression.replace(/×/g, "*").replace(/÷/g, "/").replace(/−/g, "-");
                if (cleanExpr === "") return;
                let evalResult = eval(cleanExpr);
                result = evalResult.toString();
                expression = result; // Allow chaining
            } catch (e) {
                result = "Error";
            }
        } else {
            // Prevent multiple operators if desired, but keep it simple for now
            expression += input;
        }
    }

    // Handle Clicks
    if (response.click) {
        const clickX = response.click.x * 250;
        const clickY = response.click.y * 350;

        for (const btn of BUTTONS) {
            if (clickX >= btn.x && clickX <= btn.x + btn.w &&
                clickY >= btn.y && clickY <= btn.y + btn.h) {
                handleInput(btn.val);
                break;
            }
        }
    }

    // Handle Keyboard
    if (response.keyboard) {
        for (const key of response.keyboard) {
            if (key >= "0" && key <= "9") handleInput(key);
            else if (key === "+") handleInput("+");
            else if (key === "-") handleInput("-");
            else if (key === "*") handleInput("*");
            else if (key === "/") handleInput("/");
            else if (key === "Enter" || key === "=") handleInput("=");
            else if (key === "Backspace") handleInput("back");
            else if (key === "Escape") handleInput("C");
            else if (key === ".") handleInput(".");
            else if (key === "(") handleInput("(");
            else if (key === ")") handleInput(")");
        }
    }

    state.set("expression", expression);
    state.set("result", result);

    api.findById("display").setText(result || "0");
    api.findById("sub-display").setText(expression || " ");
}

// Simple eval-like for basic math since Boa might have restricted eval or we want to be safe
// But for a widget like this, eval() is often available in the engine.

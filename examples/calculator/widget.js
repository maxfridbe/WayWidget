const BUTTON_VALUES = {
    "btn-C": "C",
    "btn-paren-open": "(",
    "btn-paren-close": ")",
    "btn-div": "/",
    "btn-7": "7",
    "btn-8": "8",
    "btn-9": "9",
    "btn-mul": "*",
    "btn-4": "4",
    "btn-5": "5",
    "btn-6": "6",
    "btn-sub": "-",
    "btn-1": "1",
    "btn-2": "2",
    "btn-3": "3",
    "btn-add": "+",
    "btn-0": "0",
    "btn-dot": ".",
    "btn-back": "back",
    "btn-equal": "="
};

function update(api, timestamp, response, state, request) {
    if (request) {
        request.localClickEvents();
        request.localKeyEvents();
    }

    let expression = state.get("expression") || "";
    let result = state.get("result") || "0";

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

    // Handle Clicks with newly implemented Backend Hit Testing
    if (response.click && response.click.id) {
        const val = BUTTON_VALUES[response.click.id];
        if (val) {
            handleInput(val);
        }
    }

    // Handle Keyboard
    if (response.keyboard) {
        for (let key of response.keyboard) {
            // Remove the '+' prefix if present from engine
            if (key.startsWith("+")) key = key.slice(1);
            
            if (key >= "0" && key <= "9") handleInput(key);
            else if (key === "+" || key === "plus") handleInput("+");
            else if (key === "-" || key === "minus") handleInput("-");
            else if (key === "*" || key === "asterisk") handleInput("*");
            else if (key === "/" || key === "slash") handleInput("/");
            else if (key === "Enter" || key === "Return" || key === "=") handleInput("=");
            else if (key === "Backspace") handleInput("back");
            else if (key === "Escape") handleInput("C");
            else if (key === "." || key === "period") handleInput(".");
            else if (key === "(" || key === "parenleft") handleInput("(");
            else if (key === ")" || key === "parenright") handleInput(")");
        }
    }

    state.set("expression", expression);
    state.set("result", result);

    api.findById("display").setText(result || "0");
    api.findById("sub-display").setText(expression || " ");
}

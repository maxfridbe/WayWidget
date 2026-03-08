use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CliCall {
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResult {
    pub output: String,
    pub error: Option<String>,
}

pub fn process_cli_queue(calls: Vec<CliCall>, responses: Arc<Mutex<HashMap<String, CliResult>>>, loop_signal: smithay_client_toolkit::reexports::calloop::LoopSignal) {
    for call in calls {
        let responses = responses.clone();
        let loop_signal = loop_signal.clone();
        std::thread::spawn(move || {
            let result = Command::new("sh")
                .arg("-c")
                .arg(&call.command)
                .output();

            let cli_res = match result {
                Ok(output) => {
                    let out_str = String::from_utf8_lossy(&output.stdout).to_string();
                    let err_str = if output.status.success() {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&output.stderr).to_string())
                    };
                    CliResult { output: out_str, error: err_str }
                }
                Err(e) => {
                    CliResult { output: String::new(), error: Some(e.to_string()) }
                }
            };
            responses.lock().unwrap().insert(call.command, cli_res);
            loop_signal.wakeup();
        });
    }
}

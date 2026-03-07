use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post(String),
}

#[derive(Debug, Clone)]
pub struct HttpCall {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub method: HttpMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResult {
    pub status: u16,
    pub body: String,
    pub error: Option<String>,
}

pub fn process_http_queue(calls: Vec<HttpCall>, responses: Arc<Mutex<HashMap<String, HttpResult>>>) {
    for call in calls {
        let responses = responses.clone();
        std::thread::spawn(move || {
            let agent = ureq::Agent::new_with_defaults();
            
            let result = match &call.method {
                HttpMethod::Get => {
                    let mut rb = agent.get(&call.url);
                    for (k, v) in &call.headers {
                        rb = rb.header(k, v);
                    }
                    rb.call()
                }
                HttpMethod::Post(body) => {
                    let mut rb = agent.post(&call.url);
                    for (k, v) in &call.headers {
                        rb = rb.header(k, v);
                    }
                    rb.send(body.as_bytes())
                }
            };

            let http_res = match result {
                Ok(mut resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.body_mut().read_to_string().unwrap_or_default();
                    HttpResult { status, body, error: None }
                }
                Err(e) => {
                    let err_msg = format!("{:?}", e);
                    HttpResult { status: 0, body: String::new(), error: Some(err_msg) }
                }
            };
            responses.lock().unwrap().insert(call.url, http_res);
        });
    }
}

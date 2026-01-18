use std::collections::HashMap;
use std::fmt;
use tokio::net::TcpStream;

use chrono::{DateTime, Utc};
use std::net::SocketAddr;

pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub stream: TcpStream,
    pub remote_addr: Option<SocketAddr>,
    pub timestamp: DateTime<Utc>,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ANSI color codes
        const CYAN: &str = "\x1b[36m";
        const GREEN: &str = "\x1b[32m";
        const YELLOW: &str = "\x1b[33m";
        const BLUE: &str = "\x1b[34m";
        const MAGENTA: &str = "\x1b[35m";
        const RESET: &str = "\x1b[0m";

        let addr = self
            .remote_addr
            .map(|a| format!("{MAGENTA}{}{RESET}", a, MAGENTA = MAGENTA, RESET = RESET))
            .unwrap_or_else(|| {
                format!("{MAGENTA}unknown{RESET}", MAGENTA = MAGENTA, RESET = RESET)
            });

        write!(
            f,
            "{CYAN}[{timestamp}]{RESET} {GREEN}INFO{RESET} {addr} \"{YELLOW}{method}{RESET} {BLUE}{url}{RESET}\"\nHeaders: {:#?}\nBody: {}",
            self.headers,
            self.body,
            CYAN = CYAN,
            GREEN = GREEN,
            YELLOW = YELLOW,
            BLUE = BLUE,
            RESET = RESET,
            timestamp = self.timestamp.to_rfc3339(),
            addr = addr,
            method = self.method,
            url = self.url
        )
    }
}

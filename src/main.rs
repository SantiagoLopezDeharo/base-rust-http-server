use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc};
use tokio::time::{Duration, sleep};

mod db;
mod domain;
mod middlewares;
mod primitives;
mod routing;
use chrono::Utc;
use primitives::http::request::Request;
use routing::{init, init_routes, route};

async fn handle_connection(mut stream: TcpStream, _permit: tokio::sync::OwnedSemaphorePermit) {
    let remote_addr = stream.peer_addr().ok();
    let mut buf_reader = BufReader::new(&mut stream);
    let mut http_request = Vec::new();
    let mut line = String::new();

    let timestamp = Utc::now();

    while buf_reader.read_line(&mut line).await.unwrap() > 0 {
        let trimmed = line.trim_end().to_string();
        if trimmed.is_empty() {
            break;
        }
        http_request.push(trimmed);
        line.clear();
    }

    let (method, url, _version) = if let Some(request_line) = http_request.get(0) {
        let mut parts = request_line.split_whitespace();
        (
            parts.next().unwrap_or("").to_string(),
            parts.next().unwrap_or("").to_string(),
            parts.next().unwrap_or("").to_string(),
        )
    } else {
        ("".to_string(), "".to_string(), "".to_string())
    };

    let mut headers = HashMap::new();
    for line in http_request.iter().skip(1) {
        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }

    let mut body = String::new();
    if let Some(content_length) = headers.get("Content-Length") {
        if let Ok(len) = content_length.parse::<usize>() {
            let mut buf = vec![0u8; len];
            buf_reader.read_exact(&mut buf).await.unwrap();
            body = String::from_utf8_lossy(&buf).to_string();
        }
    }

    let mut request = Request {
        method,
        url,
        headers,
        body,
        stream,
        remote_addr,
        timestamp,
    };

    let response = route(&mut request).await;

    println!("//=====================//");
    println!("{}", request);

    request
        .stream
        .write_all(&response.to_bytes())
        .await
        .unwrap();
    let _ = request.stream.shutdown().await;
}

fn main() {
    dotenv().ok();
    println!("Hello, world!");
    init(init_routes());

    let cores = env::var("CORES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        });

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("127.0.0.1:{}", port);

    let max_connections = cores * 1024;
    let connection_limiter = std::sync::Arc::new(Semaphore::new(max_connections));

    let mut senders = Vec::with_capacity(cores);
    for _ in 0..cores {
        let (tx, mut rx) = mpsc::channel::<(TcpStream, tokio::sync::OwnedSemaphorePermit)>(1024);
        senders.push(tx);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();

            runtime.block_on(local.run_until(async move {
                while let Some((stream, permit)) = rx.recv().await {
                    tokio::task::spawn_local(handle_connection(stream, permit));
                }
            }));
        });
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let _ = db::init_pool()
            .await
            .expect("Failed to initialize DB pool");

        let listener = TcpListener::bind(&bind_addr).await.unwrap();
        let mut next = 0usize;

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    eprintln!("Accept failed: {err}");
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }
            };

            match connection_limiter.clone().try_acquire_owned() {
                Ok(permit) => {
                    if senders[next].send((stream, permit)).await.is_err() {
                        eprintln!("Worker channel closed");
                    }
                }
                Err(_) => {
                    let mut stream = stream;
                    let _ = stream
                        .write_all(
                            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .await;
                    let _ = stream.shutdown().await;
                }
            }
            next = (next + 1) % senders.len();
        }
    });
}

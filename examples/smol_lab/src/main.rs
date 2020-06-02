use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, SystemTime};

use bytes::{BufMut, BytesMut};
use futures::prelude::*;
use minihttp::{Request, Response};
use smol::{Async, Task, Timer};

use async_dup::Arc;
use async_tungstenite::tungstenite::protocol::Role;
use async_tungstenite::WebSocketStream;
use base64;
use sha1::{Digest, Sha1};

async fn process(stream: Async<TcpStream>) -> io::Result<()> {
    let mut v = vec![0u8; 16 * 1024];
    let mut input = BytesMut::new();
    let mut output = BytesMut::new();

    let stream_arc = Arc::new(stream);
    let mut stream = stream_arc.clone();

    let upgrade = '__http_loop: loop {
        match stream.read(&mut v).await? {
            0 => return Ok(()),
            n => input.extend_from_slice(&v[..n]),
        }

        '__http_while: while let Some(request) = Request::decode(&mut input)? {
            if request.path() == "/ws" && request.method() == "GET" {
                // https://github.com/snapview/tungstenite-rs/blob/v0.10.1/src/handshake/server.rs#L27
                if request.version() != 1 {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("HTTP version should be 1.1 or higher")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }

                if let None = request
                    .headers()
                    .filter(|x| x.0 == "Connection" && x.1 == "Upgrade".as_bytes())
                    .next()
                {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("No \"Connection: upgrade\" in client request")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }

                if let None = request
                    .headers()
                    .filter(|x| x.0 == "Upgrade" && x.1 == "websocket".as_bytes())
                    .next()
                {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("No \"Upgrade: websocket\" in client request")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }

                if let None = request
                    .headers()
                    .filter(|x| x.0 == "Sec-WebSocket-Version" && x.1 == "13".as_bytes())
                    .next()
                {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("No \"Sec-WebSocket-Version: 13\" in client request")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }

                let key_header = request
                    .headers()
                    .filter(|x| x.0 == "Sec-WebSocket-Key")
                    .next();
                if key_header.is_none() {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("Missing Sec-WebSocket-Key")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }
                let key = key_header.unwrap().1;

                Response::new()
                    .status_code(101, "Switching Protocols")
                    .header("Connection", "Upgrade")
                    .header("Upgrade", "websocket")
                    .header("Sec-WebSocket-Accept", convert_key(key)?.as_str())
                    .encode(&mut output);
                stream.write_all(&output).await?;
                output.clear();

                break '__http_loop "ws";
            } else if request.path() == "/sse" && request.method() == "GET" {
                if request.version() != 1 {
                    Response::new()
                        .status_code(400, "Bad Request")
                        .header("Content-Type", "text/plain")
                        .body("HTTP version should be 1.1 or higher")
                        .encode(&mut output);
                    stream.write_all(&output).await?;
                    output.clear();

                    continue '__http_while;
                }

                let mut buf = BytesMut::with_capacity(1024);
                buf.put(&b"HTTP/1.1 200 OK\r\n"[..]);
                buf.put(&b"Content-Type: text/event-stream\r\n"[..]);
                buf.put(&b"Cache-Control: no-cache\r\n"[..]);
                buf.put(&b"Connection: keep-alive\r\n"[..]);
                buf.put(&b"Access-Control-Allow-Origin: *\r\n"[..]);
                buf.put(&b"\r\n"[..]);
                buf.put(&b"retry: 10000\n\n"[..]);

                output.extend_from_slice(&buf[..]);
                stream.write_all(&output).await?;
                output.clear();

                break '__http_loop "sse";
            } else {
                Response::new()
                    .header("Content-Type", "text/plain")
                    .body("Hello, World!")
                    .encode(&mut output);
                stream.write_all(&output).await?;
                output.clear();
            }
        }
    };

    //
    assert!(input.is_empty());

    //
    println!("upgrade: {}", upgrade);

    //
    if upgrade == "ws" {
        let role = Role::Server;
        let ws_stream = WebSocketStream::from_raw_socket(stream_arc.clone(), role, None).await;
        println!("New WebSocket connection: {:?}", ws_stream);
        // https://github.com/sdroege/async-tungstenite/blob/0.5.0/examples/echo-server.rs#L48
        let (write, read) = ws_stream.split();
        read.forward(write)
            .await
            .expect("Failed to forward message");
    } else if upgrade == "sse" {
        loop {
            Timer::after(Duration::from_secs(1)).await;
            stream_arc
                .clone()
                .write_all(format!("data: {:?}\n\n", SystemTime::now()).as_bytes())
                .await?;
        }
    }

    Ok(())
}

// https://github.com/snapview/tungstenite-rs/blob/v0.10.1/src/handshake/mod.rs#L123
fn convert_key(input: &[u8]) -> Result<String, io::Error> {
    // ... field is constructed by concatenating /key/ ...
    // ... with the string "258EAFA5-E914-47DA-95CA-C5AB0DC85B11" (RFC 6455)
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.input(input);
    sha1.input(WS_GUID);
    Ok(base64::encode(&sha1.result()))
}

fn main() -> io::Result<()> {
    for _ in 0..num_cpus::get().max(1) {
        thread::spawn(|| smol::run(future::pending::<()>()));
    }

    smol::block_on(async {
        let listener = Async::<TcpListener>::bind("0.0.0.0:8080")?;
        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            Task::spawn(async move {
                let _ = process(stream).await;
            })
            .detach();
        }
        Ok(())
    })
}

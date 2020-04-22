use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;

use bytes::BytesMut;
use futures::prelude::*;
use minihttp::{Request, Response};
use smol::{Async, Task};

async fn process(mut stream: Async<TcpStream>) -> io::Result<()> {
    let mut v = vec![0u8; 16 * 1024];
    let mut input = BytesMut::new();
    let mut output = BytesMut::new();

    loop {
        match stream.read(&mut v).await? {
            0 => return Ok(()),
            n => input.extend_from_slice(&v[..n]),
        }

        while let Some(_) = Request::decode(&mut input)? {
            Response::new()
                .header("Content-Type", "text/plain")
                .body("Hello, World!")
                .encode(&mut output);
            stream.write_all(&output).await?;
            output.clear();
        }
    }
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

use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Extension, Router,
};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use tokio::{
    fs::OpenOptions,
    io::{self, AsyncReadExt, AsyncWriteExt},
};

use std::sync::{Arc, Mutex};

struct State {
    buffer: Mutex<[[u8; 80]; 1000]>,
    row: usize,
    pos: (u8, u8),
    height: usize,
    width: usize,
}

impl State {
    fn new() -> State {
        State {
            buffer: Mutex::new([[0; 80]; 1000]),
            row: 0,
            pos: (0, 0),
            height: 25,
            width: 80,
        }
    }
}

#[tokio::main]
async fn main() {
    // let mut reader = OpenOptions::new()
    //     .read(true)
    //     .write(false)
    //     .create(false)
    //     .truncate(false)
    //     .open("/dev/pts/6")
    //     .await
    //     .unwrap();
    // let mut writer = OpenOptions::new()
    //     .read(false)
    //     .write(true)
    //     .create(false)
    //     .truncate(false)
    //     .open("/dev/pts/6")
    //     .await
    //     .unwrap();
    // //let (mut reader, mut writer) = io::split(file);

    // let mut handles = Vec::new();

    // handles.push(tokio::spawn(async move {
    //     let mut stdout = io::stdout();
    //     io::copy(&mut reader, &mut stdout).await
    //     //    let mut buf = [0; 80];
    //     //    loop {
    //     //        let n = reader.read(&mut buf).await.unwrap();
    //     //        eprintln!("Read {} bytes from pts", n);
    //     //        stdout.write(&buf[..n]).await.unwrap();
    //     //        eprintln!("Wrote {} bytes to stdout", n);
    //     //    }
    // }));

    // handles.push(tokio::spawn(async move {
    //     let mut stdin = io::stdin();
    //     io::copy(&mut stdin, &mut writer).await
    //     // let mut buf = [0; 80];
    //     // loop {
    //     //     let n = stdin.read(&mut buf).await.unwrap();
    //     //     eprintln!("Read {} bytes from stdin", n);
    //     //     writer.write(&buf[..n]).await.unwrap();
    //     //     eprintln!("Wrote {} bytes to pty", n);
    //     // }
    // }));

    let app = Router::new()
        .route("/ws", get(handler))
        .layer(Extension(Arc::new(State::new())));

    //tokio::task::spawn(async move {
    axum::Server::bind(&"[::]:9999".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    //});
    // for handle in handles {
    //     handle.await.unwrap().unwrap();
    //     //handle.await.unwrap();
    // }
}

async fn handler(ws: WebSocketUpgrade, Extension(state): Extension<Arc<State>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<State>) {
    let (mut sender, receiver) = socket.split();
    tokio::spawn({
        let state = state.clone();
        async move {
            let mut reader = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .truncate(false)
                .open("/dev/pts/6")
                .await
                .unwrap();
            loop {
                let mut buf = vec![0; 80];
                let n = reader.read(&mut buf).await.unwrap();
                buf.truncate(n);
                let msg = String::from_utf8(buf).unwrap();
                eprint!("Sending message {}", msg);
                sender
                    .send(axum::extract::ws::Message::Text(msg))
                    .await
                    .unwrap();
            }
            //io::copy(&mut reader, &mut sender).await.unwrap();
        }
    });

    tokio::spawn({
        async move {
            let mut writer = OpenOptions::new()
                .read(false)
                .write(true)
                .create(false)
                .truncate(false)
                .open("/dev/pts/6")
                .await
                .unwrap();
            receiver
                .for_each(|msg| async {
                    let mut writer = writer.try_clone().await.unwrap();
                    if let Ok(msg) = msg {
                        match msg {
                            axum::extract::ws::Message::Binary(d) => {
                                writer.write(&d).await.unwrap();
                            }
                            axum::extract::ws::Message::Text(t) => {
                                eprint!("Got message {}", t);
                                writer.write(&t.as_bytes()).await.unwrap();
                                eprintln!("Forwarded data");
                            }
                            _ => {}
                        };
                    };
                })
                .await;
            // let mut buf = vec![0; 80];
            // receiver.re
            // let n = receiver.recv(&mut buf).await.unwrap();
            // buf.truncate(n);
            // sender
            //     .send(axum::extract::ws::Message::Binary(buf))
            //     .await
            //     .unwrap();
        }
    });
}

use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};

use std::net::SocketAddr;
use std::{borrow::Cow, ops::ControlFlow};

use futures::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};

#[tokio::main]
async fn main() {
    let socket = SocketAddr::from(([127, 0, 0, 1], 3000));
    let app = Router::new().route("/ws", get(ws_handler));

    println!("🚢 Rusty Ship is now listerning to {socket}");

    axum::Server::bind(&socket)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}
async fn handle_socket(socket: WebSocket, from: SocketAddr) {
    let (sender, receiver) = socket.split();

    tokio::spawn(write(sender, from));
    tokio::spawn(read(receiver, from));
}

async fn handle_message(message: Message, from: SocketAddr) -> ControlFlow<(), ()> {
    match message {
        Message::Text(m) => {
            println!("{} send the following text message: {:?}", from, m);
        }
        Message::Binary(_) => todo!(),
        Message::Ping(_) => todo!(),
        Message::Pong(_) => todo!(),
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    "{} has asked to close with code {} and provided reason `{}`",
                    from, cf.code, cf.reason
                );
            } else {
                println!("{} managed to send close without CloseFrame", from);
            }
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

async fn read(mut receiver: SplitStream<WebSocket>, from: SocketAddr) {
    while let Some(Ok(message)) = receiver.next().await {
        if handle_message(message, from).await.is_break() {
            break;
        }
    }
}

/// Write messages back using the socket
/// If an error is thrown it closes the connection
async fn write(mut sender: SplitSink<WebSocket, Message>, from: SocketAddr) {
    if sender
        .send(Message::Text(format!("Hello there client {from}!")))
        .await
        .is_err()
    {}

    println!("Closing the connection for {from}");

    if let Err(e) = sender
        .send(Message::Close(Some(CloseFrame {
            code: axum::extract::ws::close_code::NORMAL,
            reason: Cow::from("Night-night"),
        })))
        .await
    {
        println!("Unable to Close because {}, check the error", e);
    }
}
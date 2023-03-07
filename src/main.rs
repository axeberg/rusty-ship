use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::PathBuf;

use log::{debug, error, info, warn};

use anyhow::Result;

use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

use axum::handler::HandlerWithoutStateExt;
use axum::http::Response;
use axum::response::Html;
use axum::BoxError;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Host, WebSocketUpgrade,
    },
    http::{StatusCode, Uri},
    response::Redirect,
};
use axum::{response::IntoResponse, routing::get, Router};

use axum_server::tls_rustls::RustlsConfig;

use futures::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let ports = Ports {
        http: 7878,
        https: 3000,
    };

    // Spawns a second server help redirect http requests to https
    tokio::spawn(redirect_http_to_https(ports));

    // Configuration and setup for certificate and private key
    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();

    let middleware_stack = ServiceBuilder::new()
        .layer(CompressionLayer::new().gzip(true))
        .into_inner();

    let app = Router::new()
        .route("/", get(index))
        .route("/index.mjs", get(indexmjs_get))
        .route("/ws", get(ws_handler))
        .layer(middleware_stack);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    info!("🚢 Rusty Ship is now listening to {addr}");

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

/// Handles redirection of requests from http to https
async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                warn!("Failed to convert URI to HTTPS {error}");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    debug!("HTTP redirect is listening on {addr}");

    axum::Server::bind(&addr)
        .serve(redirect.into_make_service())
        .await
        .unwrap();
}

/// Constructs a HTML response for the client index.html
async fn index() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("client/index.html")
        .await
        .unwrap();
    Html(markup)
}

/// Constructs string response using preact client on HTTP/HTTPS GET for the index.mjs module
async fn indexmjs_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("client/index.mjs").await.unwrap();

    Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(markup)
        .unwrap()
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_upgrade(socket, addr))
}
async fn handle_upgrade(socket: WebSocket, from: SocketAddr) {
    info!("Successfully upgraded websocket ⤴️");
    let (sender, receiver) = socket.split();
    info!("Splitting socket 🪓");

    let mut send_task = tokio::spawn(send(sender, from));
    let mut receive_task = tokio::spawn(receive(receiver, from));

    // If any one of the tasks run to completion, we abort the other.
    tokio::select! {
        /* _ = (&mut send_task) => receive_task.abort(), */
        _ = (&mut receive_task) => send_task.abort(),
    };
}

/// Read messages sent over the socket
/// If an error is thrown it closes the connection
async fn receive(mut receiver: SplitStream<WebSocket>, from: SocketAddr) {
    while let Some(Ok(Message::Text(message))) = receiver.next().await {
        let _ = handle_message(axum::extract::ws::Message::Text(message), from)
            .await
            .is_break();
    }
}

/// Write messages back using the socket
/// If an error is thrown it closes the connection
async fn send(mut sender: SplitSink<WebSocket, Message>, from: SocketAddr) {
    if let Err(e) = sender
        .send(Message::Text(format!("Hello there client {from}!")))
        .await
    {
        error!("Unable to send because {}, check the error", e);
    }
}

async fn handle_message(message: Message, from: SocketAddr) -> ControlFlow<(), ()> {
    match message {
        Message::Text(m) => {
            info!("{} send the following text message: {:?}", from, m);
        }
        Message::Binary(_) => todo!(),
        Message::Ping(_) => todo!(),
        Message::Pong(_) => todo!(),
        Message::Close(c) => {
            if let Some(cf) = c {
                info!(
                    "{} has asked to close with code {} and provided reason `{}`",
                    from, cf.code, cf.reason
                );
            } else {
                error!("{} managed to send close without CloseFrame", from);
            }
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

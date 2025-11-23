mod config;
mod modules;
mod shared;

use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::prelude::*;

#[handler]
async fn hello() -> &'static str {
    "Hello World"
}

#[tokio::main]
async fn main() {
    let host = "0.0.0.0";
    let port = 4433;

    tracing_subscriber::fmt().init();

    let cert = include_bytes!("../certs/server.crt").to_vec();
    let key = include_bytes!("../certs/server.pk8").to_vec();

    let router = Router::new().get(hello);

    let config = RustlsConfig::new(Keycert::new().cert(cert.as_slice()).key(key.as_slice()));

    let listener = TcpListener::new((host, port)).rustls(config.clone());

    let acceptor = QuinnListener::new(config.build_quinn_config().unwrap(), (host, port))
        .join(listener)
        .bind()
        .await;

    Server::new(acceptor).serve(router).await;
}

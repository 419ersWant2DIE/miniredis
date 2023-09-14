#![feature(impl_trait_in_assoc_type)]
use std::net::SocketAddr;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use mini_redis::{S, LogLayer};

#[volo::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();

    let host = "[::]";
    let port = 47000;
    let addr = format!("{}:{}", host, port).parse::<SocketAddr>().unwrap();
    let addr = volo::net::Address::from(addr);
    let slave_addr = vec![
        "127.0.0.1:47001".parse::<SocketAddr>().unwrap(),
        "127.0.0.1:47002".parse::<SocketAddr>().unwrap(),
        "127.0.0.1:47003".parse::<SocketAddr>().unwrap(),
    ];
    let log_path = format!(
        "log/{}_{}_{}.log",
        host,
        port,
        match slave_addr.is_empty() {
            true => "slave",
            false => "master",
        }
    );

    volo_gen::volo::example::ItemServiceServer::new(S::new(slave_addr, log_path.as_str()).await)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();
}

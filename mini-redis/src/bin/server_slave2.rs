#![feature(impl_trait_in_assoc_type)]
use std::net::SocketAddr;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use mini_redis::{S, LogLayer};

#[volo::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();

    let host = "[::]";
    let port = 47002;
    let addr = format!("{}:{}", host, port).parse::<SocketAddr>().unwrap();
    let addr = volo::net::Address::from(addr);
    let slave_addr = vec![];
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

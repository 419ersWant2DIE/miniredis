#![feature(impl_trait_in_assoc_type)]
use std::net::SocketAddr;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use mini_redis::{S, LogLayer, DEFAULT_ADDR};

#[volo::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();

    let addr: SocketAddr = DEFAULT_ADDR.parse().unwrap();
    let addr = volo::net::Address::from(addr);
    let log_path = "log/demo.log";


    volo_gen::volo::example::ItemServiceServer::new(S::new(Vec::new(), log_path).await)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();
}

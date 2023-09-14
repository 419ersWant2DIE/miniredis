#![feature(impl_trait_in_assoc_type)]
use std::{
    net::SocketAddr,
    env,
};
// use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use mini_redis::{S, LogLayer};

#[volo::main]
async fn main() {
    // tracing_subscriber::registry().with(fmt::layer().with_writer(std::io::stdout)).init();

    let args = env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    if args.len() < 3 {
        panic!("Usage: {} <host> <port> [slave_addr]", args[0]);
    }

    let host = args.get(1).unwrap();
    let port = args.get(2).unwrap();
    let slave_addr = args[3..]
        .to_vec()
        .iter()
        .map(|s| s.parse::<SocketAddr>().unwrap())
        .collect::<Vec<_>>();

    let addr = format!("{}:{}", host, port).parse::<SocketAddr>().unwrap();
    let addr = volo::net::Address::from(addr);
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

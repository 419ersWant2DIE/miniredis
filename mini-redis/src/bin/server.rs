#![feature(impl_trait_in_assoc_type)]
use std::{
    net::SocketAddr,
    env,
};

use mini_redis::{S, LogLayer};
use volo_gen::volo::example::GetItemRequest;

#[volo::main]
async fn main() {
    tracing_subscriber::fmt::init();

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
    let is_master = !slave_addr.is_empty();

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

    let server = S::new(slave_addr, log_path.as_str()).await;

    let log_file = server.log_file.clone();
    let op_tx = match is_master {
        true => Some(server.op_tx.as_ref().unwrap().clone()),
        false => None,
    };

    volo_gen::volo::example::ItemServiceServer::new(server)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();

    // If this is a master, we need to wait for all the requests to be processed
    // For exapmle, we need to wait for all requests are broadcasted to slaves
    if let Some(op_tx) = op_tx {
        tracing::info!("Server {}:{} is closing spawned tasks by using broadcast channel", host, port);
        match op_tx.lock().unwrap().send(GetItemRequest {
            opcode: 255,
            key_channal: "".into(),
            value_message: "".into(),
            txn_id: None,
        }) {
            Ok(_) => tracing::info!("Server {}:{} is closed spawned tasks successfully", host, port),
            Err(e) => tracing::error!("Server {}:{} is closed spawned tasks failed: {}", host, port, e),
        };
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Sync log file to disk to avoid data loss
    tracing::info!("Server {}:{} is syncing log file", host, port);
    let result = log_file.lock().await.sync_all().await;
    match result {
        Ok(_) => tracing::info!("Sync log file successfully"),
        Err(e) => tracing::error!("Sync log file failed: {}", e),
    };

    tracing::info!("Server {}:{} is closed", host, port);
    
}

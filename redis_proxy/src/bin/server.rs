#![feature(impl_trait_in_assoc_type)]

use std::net::SocketAddr;
use redis_proxy::LogLayer;

use redis_proxy::{S, DEFAULT_ADDR};

#[volo::main]
async fn main() {
    // 创建存储ip的向量
    let mut ip_vec: Vec<String> = Vec::new();
    ip_vec.push("[::]:56342".into());
    ip_vec.push("[::]:56343".into());
    
    // 创建一个新的服务
    let server = S::new();

    // 根据ip创建客户端，并将其存入server中
    for ip in ip_vec {
        server.sb.write().unwrap().master_redis.push({
            let addr: SocketAddr = ip.parse().unwrap();
            volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                .layer_outer(LogLayer)
                .address(addr)
                .build()
        });
    }


    let addr: SocketAddr = DEFAULT_ADDR.parse().unwrap();
    let addr = volo::net::Address::from(addr);

    volo_gen::volo::example::ItemServiceServer::new(server)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();
}

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

    // 创建存储从节点ip的向量，slave_ip[index]为ip_vec[index]对应的从节点ip向量
    let mut slave_ip: Vec<Vec<String>> = Vec::new();

    slave_ip.push(Vec::new());
    slave_ip.push(Vec::new());

    slave_ip[0].push("[::]:63419".into());
    slave_ip[1].push("[::]:63420".into());
    
    // 创建一个新的服务
    let server = S::new();

    // 根据ip创建客户端，并将其存入server中
    for ip in ip_vec {
        {
            server.masters.write().unwrap().push({
                {
                    let addr: SocketAddr = ip.parse().unwrap();
                    volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                        .layer_outer(LogLayer)
                        .address(addr)
                        .build()
                }
            });
        }
    }

    let mut index: usize = 0;
    for slave in slave_ip {
        // 为这个主节点创建从节点客户端向量
        { server.slaves.write().unwrap().push(Vec::new()) } ;
        // 将所有从节点客户端加到上述向量中
        for ip in slave {
            
            {
                server.slaves.write().unwrap()[index].push({
                    let addr: SocketAddr = ip.parse().unwrap();
                    volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                        .layer_outer(LogLayer)
                        .address(addr)
                        .build()
                });
            }
        }
        index = index + 1;
    }


    let addr: SocketAddr = DEFAULT_ADDR.parse().unwrap();
    let addr = volo::net::Address::from(addr);

    volo_gen::volo::example::ItemServiceServer::new(server)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();
}

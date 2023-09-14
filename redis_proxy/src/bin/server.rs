#![feature(impl_trait_in_assoc_type)]

use std::net::SocketAddr;
use redis_proxy::LogLayer;
use std::env;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use redis_proxy::S;

#[volo::main]
async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();
    // 获得命令行参数
    let args: Vec<String> = env::args().collect();
    
    // 获得本机的ip地址
    let proxy_addr = args[1].clone();

    // 创建存储主从节点的ip的向量
    let mut master_ip: Vec<String> = Vec::new();
    let mut slave_ip : Vec<Vec<String>> = Vec::new();


    // 获得主从节点的ip
    let mut index: usize = 2;
    while index < args.len() {
        if args[index] == "-n" {
            index = index + 1;
        }
        // 获得主节点的ip
        master_ip.push(args[index].clone());
        index = index + 1;

        // 获得这个主节点对应的从节点ip
        slave_ip.push(Vec::new());
        while index < args.len() && args[index] != "-n" {
            slave_ip[master_ip.len() - 1].push(args[index].clone());
            index = index + 1;
        }
    }
    
    // 创建一个新的服务
    let server = S::new();

    // 根据ip创建客户端，并将其存入server中
    for ip in master_ip {
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


    let addr: SocketAddr = proxy_addr.parse().unwrap();
    let addr = volo::net::Address::from(addr);

    volo_gen::volo::example::ItemServiceServer::new(server)
        .layer_front(LogLayer)
        .run(addr)
        .await
        .unwrap();
}

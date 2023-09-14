use volo_gen::volo::example::GetItemRequest;
use std::collections::HashMap;
use std::net::SocketAddr;
use volo_gen;
use mini_redis::RedisClient;

mod common;

#[tokio::main]
async fn main() {
    let range_ = 500;
    let mut map : HashMap<String, String> = HashMap::new(); 
    for i in 1..range_ + 1 {
        map.insert(i.to_string(), common::rand_str());
    }

    let master = RedisClient::new("127.0.0.1:45000".parse::<SocketAddr>().unwrap());
    let slave = RedisClient::new("127.0.0.1:45001".parse::<SocketAddr>().unwrap());


    // test set for slave, expect to be failed
    for (key, value) in map.iter() {
        let result = slave.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_err());
    }

    // test set for master, expect to be OK
    for (key, value) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
    }

    // test get for slave, expect to be OK
    for (key, value) in map.iter() {
        let result = slave.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, value.clone());
    }

    // test get for master, expect to be OK
    for (key, value) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, value.clone());
    }
}

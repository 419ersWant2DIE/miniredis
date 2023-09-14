use volo_gen::volo::example::GetItemRequest;
use std::collections::HashMap;
use std::net::SocketAddr;
use volo_gen;
use mini_redis::RedisClient;

mod common;

#[tokio::main]
async fn main() {
    let range_ = 2000;
    let mut map : HashMap<String, String> = HashMap::new(); 
    for i in 1..range_ + 1 {
        map.insert(i.to_string(), common::rand_str());
    }

    let proxy = RedisClient::new("127.0.0.1:41000".parse::<SocketAddr>().unwrap());

    // test set through proxy, expect to be OK
    for (key, value) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
    }

    // test get from proxy, expect to be the same as the value set before
    for (key, value) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, value.clone());
    }

    // test del through proxy, expect to be OK
    for (key, _) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 2,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "1".to_string());
    }

    // test del through proxy, expect to be OK
    for (key, _) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 2,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "0".to_string());
    }

    // test get from proxy, expect to be empty strings as "(nil)"
    for (key, value) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "(nil)".to_string());
    }

    // test set through proxy, expect to be OK
    for (key, value) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
    }

    // test get from proxy, expect to be the same as the value set before
    for (key, value) in map.iter() {
        let result = proxy.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, value.clone());
    }
}

use volo_gen::volo::example::GetItemRequest;
use std::collections::HashMap;
use std::net::SocketAddr;
use volo_gen;
use mini_redis::RedisClient;
use ansi_term::Colour::Green;
use std::io::Write;

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
    print!("1. test set for slave, expect to be failed: ");
    std::io::stdout().flush().unwrap();
    for (key, value) in map.iter() {
        let result = slave.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_err());
    }
    println!("{}", Green.paint("PASS"));

    // test set for master, expect to be OK
    print!("2. test set for master, expect to be OK: ");
    std::io::stdout().flush().unwrap();
    for (key, value) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 1,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
    }
    println!("{}", Green.paint("PASS"));

    // test get for slave, expect to be OK
    print!("3. test get for slave, expect to be OK: ");
    std::io::stdout().flush().unwrap();
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
    println!("{}", Green.paint("PASS"));

    // test get for master, expect to be OK
    print!("4. test get for master, expect to be OK: ");
    std::io::stdout().flush().unwrap();
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
    println!("{}", Green.paint("PASS"));

    // test del for slave, expect to be failed
    print!("5. test del for slave, expect to be failed: ");
    std::io::stdout().flush().unwrap();
    for (key, _) in map.iter() {
        let result = slave.get_item(GetItemRequest {
            opcode: 2,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_err());
    }
    println!("{}", Green.paint("PASS"));

    // test del for master, expect to be OK
    print!("6. test del for master, expect to be OK: ");
    std::io::stdout().flush().unwrap();
    for (key, _) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 2,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "1".to_string());
    }
    println!("{}", Green.paint("PASS"));

    // test del repeatly for master, expect to be OK
    print!("7. test del repeatly for master, expect to be OK: ");
    std::io::stdout().flush().unwrap();
    for (key, _) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 2,
            key_channal: key.clone().into(),
            value_message: "".into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "0".to_string());
    }
    println!("{}", Green.paint("PASS"));

    // test get for slave, expect to be empty strings as "(nil)"
    print!("8. test get for slave, expect to be empty strings as \"(nil)\": ");
    std::io::stdout().flush().unwrap();
    for (key, value) in map.iter() {
        let result = slave.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "(nil)".to_string());
    }
    println!("{}", Green.paint("PASS"));

    // test get for master, expect to be empty strings as "(nil)"
    print!("9. test get for master, expect to be empty strings as \"(nil)\": ");
    std::io::stdout().flush().unwrap();
    for (key, value) in map.iter() {
        let result = master.get_item(GetItemRequest {
            opcode: 0,
            key_channal: key.clone().into(),
            value_message: value.clone().into(),
            txn_id: None,
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value_message, "(nil)".to_string());
    }
    println!("{}", Green.paint("PASS"));
}

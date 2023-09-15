use lazy_static::lazy_static;
use std::collections::HashMap;
use std::net::SocketAddr;
use volo_gen;
use std::io::Write;
use ansi_term::Colour::Green;

mod common;

lazy_static! {
    static ref CLIENT: volo_gen::volo::example::ItemServiceClient = {
            let addr: SocketAddr = "127.0.0.1:45000".parse().unwrap();
            volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                .address(addr)
                .build()
    };
}

pub async fn func(opcode:&i32, key:&str, value: Option<&String>)  {
    match opcode {
        0 => {
            let res = get(key).await;
            match res {
                Some( str) => {
                    assert!(value != None);
                    assert_eq!(str, value.unwrap().to_string());
                },
                None => {
                    assert!(value == None);
                }
            }
        },
        1 => set(key, value).await,
        2 => del(key).await,
        _ => {}
    };
}

async fn get(key:&str) -> Option<String>{
    let req = volo_gen::volo::example::GetItemRequest {
        opcode: 0,
        key_channal : key.to_string().into(),
        value_message : " ".into(),
        txn_id: None,
    };
    let resp = CLIENT.get_item(req).await;
    match resp {
        Ok(info) => {
            if info.success {
                Some(info.value_message.into_string())
                //println!("Get value of key = {} : value = {}",info.key_channal, info.value_message);
            }else {
                None
            }
        },
        Err(_) => None
    }
}

async fn set(key: &str, value: Option<&String>) {
    let req = volo_gen::volo::example::GetItemRequest {
        opcode: 1,
        key_channal : key.to_string().into(),
        value_message : value.unwrap().to_string().into(),
        txn_id: None,
    };
    let resp = CLIENT.get_item(req).await;
    match resp {
        Ok(info) => {
            assert_eq!(info.success, true);
        },
        Err(_) => {assert!(false)}
    };
}

async fn del(key:&str) {
    let req = volo_gen::volo::example::GetItemRequest {
        opcode: 2,
        key_channal : key.to_string().into(),
        value_message : " ".into(),
        txn_id: None,
    };
    let resp = CLIENT.get_item(req).await;
    match resp {
        Ok(info) => {
            assert_eq!(info.success, true);
        },
        Err(_) => {assert!(false)}
    };
}

#[tokio::main]
async fn main() {
    let range_ = 500;
    let mut map : HashMap<String,String> = HashMap::new(); 
    for i in 1..range_ + 1 {
        map.insert(i.to_string(), common::rand_str());
    }
    //get
    print!("1. get the value of a key that does not exist: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&0, i.to_string().as_str(),None).await;
    }
    println!("{}", Green.paint("PASS"));

    //set
    print!("2. set the value of a key: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&1, i.to_string().as_str(),map.get(&i.to_string())).await;
    }
    println!("{}", Green.paint("PASS"));

    //get
    print!("3. get the value of a key and compare it to the value that was set: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&0, i.to_string().as_str(),map.get(&i.to_string())).await;
    }
    println!("{}", Green.paint("PASS"));

    //del
    print!("4. del the value of the set keys: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&2, i.to_string().as_str(),None).await;
    }
    println!("{}", Green.paint("PASS"));

    //get
    print!("5. get the value of a key that was deleted: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&0, i.to_string().as_str(),None).await;
    }
    println!("{}", Green.paint("PASS"));

    //set
    print!("6. set the value of a key that does not exist: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&1, i.to_string().as_str(),map.get(&i.to_string())).await;
    }
    println!("{}", Green.paint("PASS"));

    let mut a = String::new();
    println!("请按任意键继续...");
    std::io::stdin().read_line(&mut a).expect("zzxsb");

    //get
    print!("7. get the value of a key that is recovered from the AOF log: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&0, i.to_string().as_str(),map.get(&i.to_string())).await;
    }
    println!("{}", Green.paint("PASS"));

    //del
    print!("8. del the value of keys that is recovered from the AOF log: ");
    std::io::stdout().flush().unwrap();
    for i in 1..range_ + 1 {
        func(&2, i.to_string().as_str(),None).await;
    }
    println!("{}", Green.paint("PASS"));

    //get
    print!("9. get the value of a key that was deleted: ");
    for i in 1..range_ + 1 {
        func(&0, i.to_string().as_str(),None).await;
    }
    println!("{}", Green.paint("PASS"));

}


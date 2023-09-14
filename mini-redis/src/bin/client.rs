use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::env;
use mini_redis::LogLayer;
use std::io;
use std::io::Write;
// use volo_gen::volo::example::{GetItemResponse, get_item};
use mini_redis::OPCode;

static mut ADDR_STR: String = String::new();

lazy_static! {
    static ref CLIENT: volo_gen::volo::example::ItemServiceClient = {
        unsafe {
            let addr: SocketAddr = ADDR_STR.parse().unwrap();
            volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                .layer_outer(LogLayer)
                .address(addr)
                .build()
        }
    };
}

#[volo::main]
async fn main() {
    // 获取命令行参数，其为server的IP地址
    let args: Vec<String> = env::args().collect();
    unsafe { ADDR_STR = args[1].clone(); }
    tracing_subscriber::fmt::init();

    // 判断当前是否在subscribe状态，若在，则会直接进入无限循环，监听publish程序
    let mut is_subscribe: bool = false;
    let mut channel_name: String = String::new();
    let mut txn_id: usize = usize::MAX;
    let mut watch_id: String = String::new();

    loop {
        if is_subscribe {
            let subscribe_resp = CLIENT.get_item(
                volo_gen::volo::example::GetItemRequest {
                    opcode: 4,
                    key_channal: channel_name.clone().into(),
                    value_message: " ".into(),
                    txn_id: None,
                }).await;
            match subscribe_resp {
                Ok(info) => {
                    println!("{}", info.value_message);
                },
                Err(e) => tracing::error!("{:?}", e),
            }
            continue;
        }
        print!("mini-redis>  ");
        let _ = io::stdout().flush();
        // 读入传入的命令
        let mut buf: String = String::new();
        let _ = std::io::stdin().read_line(&mut buf).unwrap();
        let buf: String = buf.trim().into();
        // 将读入的命令按照空格分裂成字符串向量
        let command: Vec<String> = parse_command(&buf);
        if command.len() == 0 {
            println!("error: The command is empty");
            continue;
        }
        let mut req = volo_gen::volo::example::GetItemRequest {
            opcode: 0,
            key_channal: " ".into(),
            value_message: "pong".into(),
            txn_id: match txn_id == usize::MAX {
                true => match watch_id.is_empty() {
                    true => None,
                    false => Some(watch_id.clone().into()),
                },
                false => Some(txn_id.to_string().into()),
            },
        };
        // 判断输入的命令，设置req
        match command[0].to_lowercase().as_str() {
            "exit" => {
                // 退出
                println!("Goodbye!");
                break;
            }
            "get" => {
                // get命令，则第二个参数是要搜索的key.
                req.opcode = 0;
                if command.len() < 2 {
                    println!("Usage: get <key>");
                    continue;
                }
                req.key_channal = command[1].clone().into();
            }
            "set" => {
                // set命令，则第二个参数为要设置的key，第三个参数为要设置的值
                if command.len() < 3 {
                    println!("Usage: set <key> <value>");
                    continue;
                }
                req.opcode = 1;
                req.key_channal = command[1].clone().into();
                req.value_message = command[2].clone().into();
            }
            "del" => {
                // del命令，则第二个参数为要删去的key
                if command.len() < 2 {
                    println!("Usage: del <key>");
                    continue;
                }
                req.opcode = 2;
                req.key_channal = command[1].clone().into();
            }
            "ping" => {
                // ping命令
                if command.len() > 2 {
                    println!("Usage: ping [message]");
                    continue;
                }
                req.opcode = 3;
                // 要是有message要返回message
                req.value_message = match command.len() > 1 {
                    true => command[1].clone().into(),
                    false => "pong".into(),
                }
            }
            "subscribe" => {
                if command.len() < 2 {
                    println!("Usage: subscribe <channal_name> ");
                    continue;
                }
                if req.txn_id.is_some() {
                    println!("Subscribe Error: Already have a transaction");
                    continue;
                }
                is_subscribe = true;
                req.opcode = 4;
                req.key_channal = command[1].clone().into();
                channel_name = command[1].clone();
                println!("The message is as follow: ");
            }
            "publish" => {
                if command.len() < 3 {
                    println!("Usage: publish <channel_name> <message>");
                    continue;
                }
                if req.txn_id.is_some() {
                    println!("Publish Error: Already have a transaction");
                    continue;
                }
                req.opcode = 5;
                req.key_channal = command[1].clone().into();
                req.value_message = command[2].clone().into();
            }
            "multi" => {
                if command.len() > 1 {
                    println!("Usage: multi");
                    continue;
                }
                req.txn_id = match watch_id.is_empty() {
                    true => None,
                    false => Some(watch_id.clone().into()),
                };
                req.opcode = 200;
            }
            "exec" => {
                if command.len() > 1 {
                    println!("Usage: exec");
                    continue;
                }
                req.opcode = 201;
            }
            "watch" => {
                if command.len() < 2 {
                    println!("Usage: watch <key>");
                    continue;
                }
                if !watch_id.is_empty() {
                    println!("Transaction Error: Already have a watch");
                    continue;
                }
                req.opcode = 202;
                req.key_channal = command[1].clone().into();
            }
            _ => {
                println!("Can't not find the command: {}", command[0]);
                continue;
            }
        } 

        // 将信息传递出去并得到返回的结果
        let resp = CLIENT.get_item(req).await;
        match resp {
            Ok(info) => {
                match OPCode::from(info.opcode) {
                    OPCode::GET => {
                        println!("{}", info.value_message);
                    }
                    OPCode::SET => {
                        println!("{}", info.value_message);
                    }
                    OPCode::DEL => {
                        println!("{}", info.value_message);
                    }
                    OPCode::PING => {
                        if info.success {
                            println!("{}", info.value_message);
                        } else {
                            println!("The connect is fail");
                        }
                    }
                    OPCode::SUBSCRIBE => {
                        if info.success {
                            println!("{}", info.value_message);
                        }
                        else {
                            println!("no publish");
                        }
                    }
                    OPCode::PUBLISH => {
                        // 若使用publish，则value_message会返回subcribe的数量
                        let message: String = info.value_message.clone().into();
                        let v : Vec<char> = message.chars().collect();
                        if info.success {
                            println!("publish success. The number of subscriber is {}", get_num(&v));
                        }
                        else {
                            println!("No subscriber found");
                        }
                    }
                    OPCode::MULTI => {
                        if info.success {
                            txn_id = info.key_channal.parse().unwrap();
                            println!("{}", info.value_message);
                        } else {
                            println!("Transaction Error: {}", info.value_message);
                        }
                    }
                    OPCode:: EXEC => {
                        if info.success {
                            println!("{}", info.value_message);
                        } else {
                            println!("Transaction Error: {}", info.value_message);
                        }
                        txn_id = usize::MAX;
                        watch_id = String::new();
                    }
                    OPCode::WATCH => {
                        if info.success {
                            watch_id = info.key_channal.clone().into();
                            println!("{}", info.value_message);
                        } else {
                            println!("Transaction Error: {}", info.value_message);
                        }
                    }
                    _ => {
                        println!("error: The opcode is error");
                    }
                }
            },
            Err(e) => tracing::error!("{:?}", e),
        }
    }
}

fn parse_command(buf: &String) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let v1: Vec<&str> = buf.split(" ").collect();
    for s in v1 {
        v.push(s.into());
    }
    v
}

fn get_num(v: &Vec<char>) -> i32 {
    let mut index = 0;
    let mut res = 0;
    while index < v.len() {
        res = res * 10 + (v[index] as i32 - '0' as i32);
        index += 1;
    }
    res
}

#![feature(impl_trait_in_assoc_type)]
use std::{
    collections::{HashMap, VecDeque, HashSet},
    sync::{RwLock, Arc, Mutex},
    net::SocketAddr,
};
use tokio::{
    sync::{
        broadcast,
        Mutex as AsyncMutex,
    },
    fs::{File, OpenOptions},
    io::{AsyncWriteExt, AsyncReadExt},
};
use anyhow::Error;

// the enum for opcode
#[derive(PartialEq, Eq)]
pub enum OPCode {
    GET = 0,
    SET = 1,
    DEL = 2,
    PING = 3,
    SUBSCRIBE = 4,
    PUBLISH = 5,
    SETMASTER = 100,
    DELMASTER = 101,
    MULTI = 200,
    EXEC = 201,
    WATCH = 202,
    NOTDEFINED = 255,
}

// impl from for opcode
impl From<i32> for OPCode {
    fn from(item: i32) -> Self {
        match item {
            0 => OPCode::GET,
            1 => OPCode::SET,
            2 => OPCode::DEL,
            3 => OPCode::PING,
            4 => OPCode::SUBSCRIBE,
            5 => OPCode::PUBLISH,
            100 => OPCode::SETMASTER,
            101 => OPCode::DELMASTER,
            200 => OPCode::MULTI,
            201 => OPCode::EXEC,
            202 => OPCode::WATCH,
            _ => OPCode::NOTDEFINED,
        }
    }
}

// TxnQueue is used to store the transaction task
struct TxnQueue {
    watch_id: Option<String>,   // store the watch_id to check if the watch key has been changed
    txn_queue: VecDeque<volo_gen::volo::example::GetItemRequest>,
}

impl TxnQueue {
    fn new(watch_id: Option<String>) -> TxnQueue {
        TxnQueue {
            watch_id,
            txn_queue: VecDeque::new(),
        }
    }

    fn push(&mut self, req: volo_gen::volo::example::GetItemRequest) {
        self.txn_queue.push_back(req);
    }

    fn pop(&mut self) -> Option<volo_gen::volo::example::GetItemRequest> {
        self.txn_queue.pop_front()
    }

    fn get_watch_id(&self) -> Option<String> {
        self.watch_id.clone()
    }
}

// package the redis client
pub struct RedisClient {
    client: volo_gen::volo::example::ItemServiceClient,
}

impl RedisClient {
    pub fn new(addr: SocketAddr) -> RedisClient {
        RedisClient {
            client: {
                volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                    .address(addr)
                    .build()
            }
        }
    }

    pub async fn get_item(&self, req: volo_gen::volo::example::GetItemRequest) -> ::core::result::Result<volo_gen::volo::example::GetItemResponse, Error> {
        match self.client.get_item(req).await {
            Ok(resp) => {
                tracing::info!("Get response: {:?}", resp);
                Ok(resp)
            },
            Err(e) => {
                tracing::error!("Get error: {:?}", e);
                Err(Error::from(e))
            }
        }
    }
}

pub struct S {
    is_master: bool,
    kv_pairs: Arc<RwLock<HashMap<String, String>>>,                     // store the key-value pairs
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,  // store the channel and the sender
    pub op_tx: Option<Arc<Mutex<broadcast::Sender<volo_gen::volo::example::GetItemRequest>>>>,
    pub log_file: Arc<AsyncMutex<File>>,
    watch_keys: Arc<RwLock<HashMap<String, HashSet<String>>>>,          // store the watch key and watch_id
    txn_queue: Arc<RwLock<HashMap<usize, TxnQueue>>>,                   // store the transaction task
}

impl S {
    pub async fn new(slave_addr: Vec<SocketAddr>, log_path: &str) -> S {
        let is_master = !slave_addr.is_empty();
        let kv_pairs = Arc::new(RwLock::new(HashMap::new()));
        let channels = Arc::new(RwLock::new(HashMap::new()));
        let op_tx = match is_master {
            true => Some(Arc::new(Mutex::new(broadcast::channel(16).0))),
            false => None,
        };
        let watch_keys = Arc::new(RwLock::new(HashMap::new()));
        let txn_queue = Arc::new(RwLock::new(HashMap::new()));

        // check if the log file exists
        if !std::path::Path::new(&log_path).exists() {
            std::fs::create_dir_all("log").unwrap();
        }
        let log_file = OpenOptions::new()
            .create(true)   // create the file if it does not exist
            .read(true)
            .append(true)
            .open(log_path)
            .await
            .unwrap();

        tracing::info!("Start recovery from log file");

        // recovery from log file
        let log_file = Arc::new(AsyncMutex::new(log_file));
        let mut buf = String::new();
        let _ = log_file.clone().lock().await.read_to_string(&mut buf).await;
        let mut lines = buf.lines();
        while let Some(line) = lines.next() {
            tracing::debug!("Recovery log item: {}", line);
            let log_item: Vec<&str> = line.split(" ").collect();
            match log_item[0] {
                "SET" => {
                    let key = log_item[1];
                    let value = log_item[2];
                    kv_pairs.write().unwrap().insert(key.to_string(), value.to_string());
                },
                "DEL" => {
                    let key = log_item[1];
                    kv_pairs.write().unwrap().remove(key);
                },
                _ => {
                    tracing::warn!("Invalid log item");
                }
            }
        }

        tracing::info!("Complete recovery from log file");

        // if it is master node, create the sync task to sync data to slave nodes
        if is_master {
            for addr in slave_addr {
                let operation_rx = Arc::new(AsyncMutex::new(op_tx.as_ref().unwrap().lock().unwrap().subscribe()));
                tokio::spawn(S::sync_slave(addr, operation_rx));
            }
        }

        S {
            is_master,
            kv_pairs,
            channels,
            op_tx,
            log_file,
            watch_keys,
            txn_queue,
        }
    }

    async fn sync_slave(
        slave_addr: SocketAddr,
        rx: Arc<AsyncMutex<broadcast::Receiver<volo_gen::volo::example::GetItemRequest>>>,
    ) -> Result<(), Error> {
        // create the redis client
        let slave = RedisClient::new(slave_addr);
        
        loop {
            // receive the request from broadcast channel
            let req = rx.lock().await.recv().await;
            match req {
                Ok(req) => {
                    // if the opcode is 255, it means the task is closed
                    if req.opcode == 255 {
                        break;
                    }
                    // send the request to slave node
                    let resp = slave.get_item(req).await?;
                    tracing::info!("Sync response: {:?}", resp);
                },
                Err(e) => {
                    tracing::warn!("Broadcast Error: {:?}", e);
                }
            }
        }

        tracing::info!("Slave {} sync task is closed", slave_addr);
        Ok(())
    }
}

unsafe impl Send for S {}
unsafe impl Sync for S {}

#[volo::async_trait]
impl volo_gen::volo::example::ItemService for S {
    async fn get_item(&self, _req: volo_gen::volo::example::GetItemRequest) -> ::core::result::Result<volo_gen::volo::example::GetItemResponse, ::volo_thrift::AnyhowError>{
        let mut resp = volo_gen::volo::example::GetItemResponse {
            opcode: _req.opcode,
            key_channal: _req.key_channal.clone(),
            value_message: " ".into(),
            success: false
        };
        let opcode = OPCode::from(_req.opcode);
        // check if need to push the request to transaction task queue
        if opcode != OPCode::MULTI
            && opcode != OPCode::EXEC
            && opcode != OPCode::WATCH
            && _req.txn_id.is_some()
            && _req.txn_id.clone().unwrap().parse::<usize>().is_ok()
        {
            let txn_id = _req.txn_id.unwrap();
            let txn_id = match txn_id.parse::<usize>() {
                Ok(id) => id,
                Err(_) => {
                    resp.success = false;
                    return Ok(resp);
                }
            };
            let mut txn_queue_locked = self.txn_queue.write().unwrap();
            let txn_queue = txn_queue_locked.get_mut(&txn_id).unwrap();
            txn_queue.push(volo_gen::volo::example::GetItemRequest {
                opcode: _req.opcode,
                key_channal: _req.key_channal,
                value_message: _req.value_message,
                txn_id: None,
            });
            resp.value_message = "QUEUED".into();
            resp.success = true;

            return Ok(resp);
        }
        match opcode {
            OPCode::GET => {
                let key: String = _req.key_channal.into();
                match self.kv_pairs.read().unwrap().get(&key) {
                    Some(value) => {
                        resp.value_message = value.clone().into();
                        resp.success = true;
                    },
                    None => {
                        resp.value_message = "(nil)".into();
                        resp.success = false;
                    }
                }
            }
            OPCode::SET | OPCode::SETMASTER => {
                // prevent the slave node from setting the key-value pair
                // unless the opcode is SETMASTER, which is sent by master node
                if !self.is_master && opcode == OPCode::SET {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.clone().key_channal.into();
                let val: String = _req.clone().value_message.into();
                let _ = self.log_file.lock().await.write_all(format!("SET {} {}\n", _req.key_channal, _req.value_message).as_bytes()).await;
                if let Some(watch_ids) = self.watch_keys.write().unwrap().get_mut(&key) {
                    if _req.txn_id.is_some() && watch_ids.contains(_req.txn_id.clone().unwrap().as_str()) {
                        watch_ids.retain(|x| x == &_req.txn_id.clone().unwrap().to_string());
                    } else {
                        watch_ids.clear();
                    }
                }

                self.kv_pairs.write().unwrap().insert(key, val);
                resp.value_message = "OK".into();
                resp.success = true;
                
                if let Some(ref tx) = self.op_tx {
                    let req = volo_gen::volo::example::GetItemRequest {
                        opcode: 100,    // set the opcode to 100, which is SETMASTER
                        key_channal: _req.key_channal.clone(),
                        value_message: _req.value_message.clone(),
                        txn_id: None,
                    };
                    // send the request to broadcast channel
                    let _ = tx.lock().unwrap().send(req);
                }
            }
            OPCode::DEL | OPCode::DELMASTER=> {
                // prevent the slave node from deleting the key-value pair
                if !self.is_master && opcode == OPCode::DEL {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.clone().key_channal.into();
                let is_in: bool = self.kv_pairs.read().unwrap().contains_key(&key);
                match is_in {
                    true => {
                        let _ = self.log_file.lock().await.write_all(format!("DEL {}\n", _req.key_channal).as_bytes()).await;
                        if let Some(watch_ids) = self.watch_keys.write().unwrap().get_mut(&key) {
                            if _req.txn_id.is_some() && watch_ids.contains(_req.txn_id.clone().unwrap().as_str()) {
                                watch_ids.retain(|x| x == &_req.txn_id.clone().unwrap().to_string());
                            } else {
                                watch_ids.clear();
                            }
                        }
                        
                        self.kv_pairs.write().unwrap().remove(&key);
                        resp.value_message = "1".into();
                        resp.success = true;

                        if let Some(ref tx) = self.op_tx {
                            let req = volo_gen::volo::example::GetItemRequest {
                                opcode: 101,    // set the opcode to 101, which is DELMASTER
                                key_channal: _req.key_channal.clone(),
                                value_message: _req.value_message.clone(),
                                txn_id: None,
                            };
                            // send the request to broadcast channel
                            let _ = tx.lock().unwrap().send(req);
                        }
                    },
                    false => {
                        resp.value_message = "0".into();
                        resp.success = true;
                    }
                }
            }
            OPCode::PING => {
                resp.value_message = _req.value_message;
                resp.success = true;
            }
            OPCode::SUBSCRIBE => {
                // prevent the slave node from subscribing the channel
                if !self.is_master {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.key_channal.into();
                let (mut _tx, mut rx) = broadcast::channel(500);
                let mut has_channel: bool = false;
                {
                    match {self.channels.read().unwrap().get(&key)} {
                        Some(get_tx) => {
                            has_channel = true;
                            rx = get_tx.subscribe();
                        },
                        None => {
                            
                        },
                    }
                }
                if has_channel {
                    let mes = rx.recv().await;
                    match mes {
                        Ok(m) => {
                            resp.value_message = m.clone().into();
                            resp.success = true;
                        },
                        Err(_e) => {
                            resp.success = false;
                        }
                    }
                } else {
                    {
                        self.channels.write().unwrap().insert(key, _tx);
                    }
                    let mes = rx.recv().await;
                    match mes {
                        Ok(m) => {
                            resp.value_message = m.clone().into();
                            resp.success = true;
                        },
                        Err(_e) => {
                            resp.success = false;
                        }
                    }
                }
            }
            OPCode::PUBLISH => {
                // prevent the slave node from publishing the message
                if !self.is_master {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.key_channal.into();
                if let Some(tx) = {self.channels.read().unwrap().get(&key)} {
                    let info = tx.send(_req.value_message.into_string());
                    match info {
                        Ok(num) => {
                            resp.success = true;
                            resp.value_message = num.to_string().into();
                        },
                        Err(_) => {
                            resp.success = false;
                        }
                    }
                }
                else {
                    resp.success = false;
                }
            }
            OPCode::MULTI => {
                // prevent the slave node from starting the transaction
                if !self.is_master {
                    return Err(Error::msg("The server is slave"));
                }

                // check if the txn_id is valid
                // if so, it means the transaction has some key watched
                let watch_id: Option<String> = match _req.txn_id {
                    Some(txn_id) => Some(txn_id.into()),
                    None => None,
                };

                // define the txn_id by the txn_queue
                // if the txn_queue is empty, the txn_id is 0
                // else the txn_id is the max txn_id plus 1
                let mut txn_queue_locked = self.txn_queue.write().unwrap();
                let txn_id = match txn_queue_locked.len() {
                    0 => 0,
                    _ => {
                        let the_max = self.txn_queue.read().unwrap();
                        let the_max = the_max.keys().max().unwrap();
                        *the_max + 1
                    }
                };
                txn_queue_locked.insert(txn_id, TxnQueue::new(watch_id));

                resp.key_channal = txn_id.to_string().into();
                resp.value_message = "OK".into();
                resp.success = true;
            }
            OPCode::EXEC => {
                // prevent the slave node from executing the transaction
                if !self.is_master {
                    return Err(Error::msg("The server is slave"));
                }

                // message is used to collect the messages of each request in the transaction
                let mut message = String::new();
                resp.success = true;

                // check if the txn_id is valid
                if _req.txn_id.is_none() {
                    resp.success = false;
                    resp.value_message = "The txn_id is none".into();
                    return Ok(resp);
                }
                let txn_id = _req.txn_id.unwrap().parse::<usize>().unwrap();
                let mut txn_queue_todo = self.txn_queue.write().unwrap().remove(&txn_id).unwrap();
                let mut can_run = true;

                // check if the watch key has been changed
                if let Some(watch_id) = txn_queue_todo.get_watch_id() {
                    let watch_key = watch_id.split("_").collect::<Vec<_>>()[0].to_string();
                    can_run = self.watch_keys.write().unwrap().get_mut(&watch_key).unwrap().remove(&watch_id);
                }

                // execute the transaction
                match can_run {
                    true => {
                        while let Some(req) = txn_queue_todo.pop() {
                            let result = self.get_item(req).await;
                            match result {
                                Ok(info) => {
                                    message = format!("{}\n{}", message, info.value_message).into();
                                },
                                Err(e) => {
                                    message = format!("{}\n{}", message, e.to_string()).into();
                                    resp.success = false;
                                    break;
                                }
                            }
                        }
                    }
                    false => {
                        message = "The watch key has been changed".into();
                        resp.success = false;
                    }
                }
                message = message.trim().into();
                resp.value_message = message.clone().into();
            }
            OPCode::WATCH => {
                // prevent the slave node from watching the key
                if !self.is_master {
                    return Err(Error::msg("The server is slave"));
                }

                // determine the watch_id
                // for example, to watch key "demo",
                // if no other watcher, the watch_id is "demo_0"
                // if there are already some watchers, the watch_id is "demo_n" where n is the max number plus 1
                let key = _req.key_channal.clone().to_string();
                let is_contain = self.watch_keys.read().unwrap().contains_key(&key) && self.watch_keys.read().unwrap().get(&key).unwrap().len() > 0;
                let watch_id = match is_contain {
                    true => {
                        let the_max = self.watch_keys.read().unwrap();
                        let the_max = the_max.get(&key).unwrap().iter().max().unwrap();
                        let the_max: Vec<&str> = the_max.split("_").collect();
                        let the_max = the_max[1].parse::<usize>().unwrap();
                        format!("{}_{}", key, the_max + 1)
                    }
                    false => format!("{}_0", key),
                };

                self.watch_keys
                    .write()
                    .unwrap()
                    .entry(key.clone())
                    .or_insert(HashSet::new())
                    .insert(watch_id.clone());

                resp.key_channal = watch_id.into();
                resp.value_message = "OK".into();
                resp.success = true;
            }
            OPCode::NOTDEFINED => {
                tracing::warn!("Invalic opcode");
            }
        }
        Ok(resp)
    }
}

pub struct LogLayer;

impl<S> volo::Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(self, inner: S) -> Self::Service {
        LogService(inner)
    }
}


#[derive(Clone)]
pub struct LogService<S>(S);

#[volo::service]
impl<Cx, Req, S> volo::Service<Cx, Req> for LogService<S>
where
    Req: std::fmt::Debug + Send + 'static,
    S: Send + 'static + volo::Service<Cx, Req> + Sync,
    S::Response: std::fmt::Debug,
    S::Error: std::fmt::Debug + From<Error>,
    Cx: Send + 'static,
{
    async fn call(&self, cx: &mut Cx, req: Req) -> Result<S::Response, S::Error> {
        let now = std::time::Instant::now();
        tracing::debug!("Received request {:?}", &req);
        // 中间件希望过滤掉kv对中涉及的不文明词汇，由于不文明词汇的判断比较复杂，此处演示过滤词汇"傻逼"
        let info: Vec<char> = format!("{req:?}").chars().collect();
        let mut can_send: bool = true;
        for i in 0..(info.len() - 1) {
            if info[i] == '傻' && info[i + 1] == '逼' {
                can_send = false;
                break;
            }
        }
        if can_send {
            let resp = self.0.call(cx, req).await;
            tracing::debug!("Sent response {:?}", &resp);
            tracing::info!("Request took {}ms", now.elapsed().as_millis());    
            return resp;
        }
        // panic!("命令中有敏感词“傻逼");
        Err(S::Error::from(Error::msg("命令中有敏感词'傻逼'")))
    }
}


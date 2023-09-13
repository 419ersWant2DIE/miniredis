#![feature(impl_trait_in_assoc_type)]
use std::{
    collections::HashMap,
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

pub const DEFAULT_ADDR: &str = "[::]:8080";

struct RedisClient {
    client: volo_gen::volo::example::ItemServiceClient,
}

impl RedisClient {
    fn new(addr: SocketAddr) -> RedisClient {
        RedisClient {
            client: {
                volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
                    .address(addr)
                    .build()
            }
        }
    }

    async fn get_item(&self, req: volo_gen::volo::example::GetItemRequest) -> ::core::result::Result<volo_gen::volo::example::GetItemResponse, Error> {
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
    kv_pairs: Arc<RwLock<HashMap<String, String>>>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    op_tx: Option<Arc<Mutex<broadcast::Sender<volo_gen::volo::example::GetItemRequest>>>>,
    log_file: Arc<AsyncMutex<File>>,
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

        // 检查日志文件是否存在
        if !std::path::Path::new(&log_path).exists() {
            std::fs::create_dir_all("log").unwrap();
        }
        let log_file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(log_path)
            .await
            .unwrap();

        tracing::info!("Start recovery from log file");

        // 从日志文件中恢复数据
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
        }
    }

    async fn sync_slave(
        slave_addr: SocketAddr,
        rx: Arc<AsyncMutex<broadcast::Receiver<volo_gen::volo::example::GetItemRequest>>>,
    ) -> Result<(), Error> {
        let slave = RedisClient::new(slave_addr);
        
        loop {
            let req = rx.lock().await.recv().await;
            match req {
                Ok(req) => {
                    let resp = slave.get_item(req).await?;
                    tracing::info!("Sync response: {:?}", resp);
                },
                Err(e) => {
                    tracing::error!("Sync error: {:?}", e);
                }
            }
        }
    }
}

unsafe impl Send for S {}
unsafe impl Sync for S {}

#[derive(PartialEq, Eq)]
enum OPCode {
    GET = 0,
    SET = 1,
    DEL = 2,
    PING = 3,
    SUBSCRIBE = 4,
    PUBLISH = 5,
    SETMASTER = 100,
    DELMASTER = 101,
    NOTDEFINED = 255,
}

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
            _ => OPCode::NOTDEFINED,
        }
    }
}

#[volo::async_trait]
impl volo_gen::volo::example::ItemService for S {
    async fn get_item(&self, _req: volo_gen::volo::example::GetItemRequest) -> ::core::result::Result<volo_gen::volo::example::GetItemResponse, ::volo_thrift::AnyhowError>{
        let mut resp = volo_gen::volo::example::GetItemResponse {opcode: 0, key_channal: _req.key_channal.clone(), value_message: " ".into(), success: false};
        let opcode = OPCode::from(_req.opcode);
        match opcode {
            OPCode::GET => {
                let key: String = _req.key_channal.into();
                match self.kv_pairs.read().unwrap().get(&key) {
                    Some(value) => {
                        resp.opcode = 0;
                        resp.value_message = value.clone().into();
                        resp.success = true;
                    },
                    None => {
                        resp.opcode = 0;
                        resp.success = false;
                    }
                }
            }
            OPCode::SET | OPCode::SETMASTER => {
                if !self.is_master && opcode == OPCode::SET {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.clone().key_channal.into();
                let val: String = _req.clone().value_message.into();
                let is_in: bool = self.kv_pairs.read().unwrap().contains_key(&key);
                if is_in {
                    resp.opcode = 1;
                    resp.success = false;
                }
                else {
                    let _ = self.log_file.lock().await.write_all(format!("SET {} {}\n", _req.key_channal, _req.value_message).as_bytes()).await;

                    self.kv_pairs.write().unwrap().insert(key, val);
                    resp.opcode = 1;
                    resp.success = true;
                    
                    if let Some(ref tx) = self.op_tx {
                        let req = volo_gen::volo::example::GetItemRequest { opcode: 100, key_channal: _req.key_channal.clone(), value_message: _req.value_message.clone() };
                        let _ = tx.lock().unwrap().send(req);
                    }
                }
            }
            OPCode::DEL | OPCode::DELMASTER=> {
                if !self.is_master && opcode == OPCode::DEL {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.clone().key_channal.into();
                let is_in: bool = self.kv_pairs.read().unwrap().contains_key(&key);
                match is_in {
                    true => {
                        let _ = self.log_file.lock().await.write_all(format!("DEL {}\n", _req.key_channal).as_bytes()).await;
                        
                        self.kv_pairs.write().unwrap().remove(&key);
                        resp.opcode = 2;
                        resp.success = true;

                        if let Some(ref tx) = self.op_tx {
                            let req = volo_gen::volo::example::GetItemRequest { opcode: 101, key_channal: _req.key_channal.clone(), value_message: _req.value_message.clone() };
                            let _ = tx.lock().unwrap().send(req);
                        }
                    },
                    false => {
                        resp.opcode = 2;
                        resp.success = false;
                    }
                } 
            }
            OPCode::PING => {
                resp.opcode = 3;
                resp.value_message = _req.value_message.clone();
                resp.success = true;
            }
            OPCode::SUBSCRIBE => {
                if self.op_tx.is_none() {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.key_channal.into();
                let (mut _tx, mut rx) = broadcast::channel(16);
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
                            resp.opcode = 4;
                            resp.value_message = m.clone().into();
                            resp.success = true;
                        },
                        Err(_e) => {
                            resp.opcode = 4;
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
                            resp.opcode = 4;
                            resp.value_message = m.clone().into();
                            resp.success = true;
                        },
                        Err(_e) => {
                            resp.opcode = 4;
                            resp.success = false;
                        }
                    }
                }
            }
            OPCode::PUBLISH => {
                if self.op_tx.is_none() {
                    return Err(Error::msg("The server is slave"));
                }
                let key: String = _req.key_channal.into();
                if let Some(tx) = {self.channels.read().unwrap().get(&key)} {
                    let info = tx.send(_req.value_message.into_string());
                    match info {
                        Ok(num) => {
                            resp.opcode = 5;
                            resp.success = true;
                            resp.value_message = num.to_string().into();
                        },
                        Err(_) => {
                            resp.opcode = 5;
                            resp.success = false;
                        }
                    }
                }
                else {
                    resp.opcode = 5;
                    resp.success = false;
                }
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


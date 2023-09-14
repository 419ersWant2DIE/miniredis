#![feature(impl_trait_in_assoc_type)]
use std::sync::RwLock;
use std::sync::Arc;
use volo_gen::volo::example::ItemServiceClient;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use rand::Rng;
use log;


use anyhow::{Error, Ok};

// pub const DEFAULT_ADDR: &str = "[::]:8080";

pub struct S {
	pub masters: Arc<RwLock<Vec<ItemServiceClient>>>,
	pub slaves: Arc<RwLock<Vec<Vec<ItemServiceClient>>>>,
}

impl S {
	pub fn new() -> S {
		S {
			masters: Arc::new(RwLock::new(Vec::new())),
			slaves: Arc::new(RwLock::new(Vec::new())),
		}
	}
}

unsafe impl Send for S {}
unsafe impl Sync for S {}

#[volo::async_trait]
impl volo_gen::volo::example::ItemService for S {
	async fn get_item(&self, _req: volo_gen::volo::example::GetItemRequest) -> ::core::result::Result<volo_gen::volo::example::GetItemResponse, ::volo_thrift::AnyhowError>{
		// 创建一个hash
		let mut hash = DefaultHasher::new();

		// 获得hash值
		let hash_code = { 
			_req.key_channal.clone().as_str().hash(&mut hash);
			hash.finish()
		};
		// 获得主节点的个数
		let master_num: usize =  { self.masters.read().unwrap().len() } ;

		// 获得将要访问的节点的id
		let master_id = (hash_code as usize) % master_num; 

		// 过滤主节点同步操作
		if _req.opcode == 100 || _req.opcode == 101 {
			return Err(::volo_thrift::AnyhowError::from(anyhow::Error::msg("Can't not handle master operations.")));
		}

		// 如果是ping操作，直接返回相关信息
		if _req.opcode == 3 {
			return Ok(volo_gen::volo::example::GetItemResponse {
				opcode: 3,
				key_channal: _req.key_channal.clone(),
				value_message: _req.value_message.clone(),
				success: true
			});
		}

		// 获得访问节点的客户端，若为get操作，则从
		let rpc_cli = match  _req.opcode == 0 {
			true => {
				// 获得对应集群的节点数量
				let mut node_num = { self.slaves.read().unwrap()[master_id].len() };
				node_num = node_num + 1;
				// 生成随机数
				let mut rng = rand::thread_rng();
				let mut node_id: usize = rng.gen();
				// 将随机数对节点数量做模
				node_id = node_id % node_num;
				if node_id == node_num - 1 {
					// get主节点
					log::info!("{}", format!("master {}", master_id));
					self.masters.read().unwrap()[master_id].clone()
				} else {
					// get从节点
					log::info!("{}", format!("master {} slave {}", master_id, node_id));
					self.slaves.read().unwrap()[master_id][node_id].clone()
				}

			},
			false => {
				log::info!("{}", format!("master {}", master_id));
				self.masters.read().unwrap()[master_id].clone()
			},
		};
		match rpc_cli.get_item(_req).await {
			::core::result::Result::Ok(resp) => {
				Ok(resp)
			},
			::core::result::Result::Err(e) => {
				Err(::volo_thrift::AnyhowError::from(anyhow::Error::msg(e)))
			}
		}
		
	}
}

// fn get_string(num: u8) -> String {
// 	let mut num: u8 = num;
// 	let mut res = String::new();
// 	let mut pow: u8 = 1;
// 	while pow <= num {
// 		pow *= 10;
// 	}
// 	pow /= 10;
// 	while pow != 0 {
// 		res.push((num / pow + '0' as u8) as char);
// 		num = num % pow;
// 		pow = pow / 10;
// 	}
// 	res
// }

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


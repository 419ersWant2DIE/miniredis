# miniredis

## 编译运行方法

需要分别编译 proxy 节点的工程文件 `redis_proxy/` 和redis节点的工程文件 `mini_redis/`。

编译 proxy 节点工程文件

```shell
cd redis_proxy/
cargo update
cargo build
```

编译 redis 节点的工程文件

```shell
cd mini-redis/
cargo update
cargo build
```

编辑配置文件 `redis.conf`，配置文件格式如下所示

```shell
# 定义 redis 集群的 ip 地址和端口，每行代表一个主从 redis 架构
# 多行表示整个集群有多个主从架构，每行中第一个为主节点，后续节点为从节点
# 一个群组只能有一个主服务器，且至少含有一个从服务器
# 此外群组数目、每个群组服务器数目不做限制
[Server]
[::]:45000, [::]:45001, [::]:45002
[::]:46000, [::]:46001, [::]:46002
# 定义了 proxy 的 ip 和端口
[Proxy]
[::]:41000
```

使用上述配置文件启动整个系统

```shell
# 回到工程根目录
./start_present.sh  # cargo run
./start_test.sh     # use release version
```

此时整个服务redis集群完成启动

## 连接集群进行访问

使用 redis 节点的工程文件 `mini_redis/` 中带有的 client 即可进行访问

```shell
cd mini-redis/
cargo run --bin client 127.0.0.1:41000
```

运行时携带了命令行参数，其为要连接的服务端的 ip 地址，例如上例为实例连接 proxy 节点。运行后服务端会进入一个交互端口，会出现以下提示符，直接在其后输入命令即可
```
mini-redis> 
```

## 测试

已有三个测试，可以进入 `mini-redis/` 目录下运行

```shell
cd mini-redis/
rm log/*
cargo run --example test_aof    # 中途需要重启服务器，即在启动脚本
                                # start_test.sh/start_present.sh 的终端中按下 ctrl-c
cargo run --example test_master_slave
cargo run --example test_proxy
```

### 附录

#### 指令

##### get

其使用格式为
```
get <key>
```

若存在，则会返回其对应的值，例如
```s
mini-redis>  get 123
2023-09-11T16:34:49.176474Z  INFO mini_redis: Request took 1ms
456
```

不存在则会返回 `(nil)` 表示空

```s
mini-redis>  get 3
2023-09-11T16:40:45.471091Z  INFO mini_redis: Request took 2ms
(nil)
```

##### set

其使用格式为
```
set <key> <value>
```

若数据库中原来不存在键值，则插入成功
```s
mini-redis>  set 456 789
2023-09-14T15:53:18.326959Z  INFO mini_redis: Request took 5ms
OK
```

##### del

其使用格式为
```
del <key>
```

其会返回删除的值的个数
```s
mini-redis>  del 456
2023-09-14T15:54:09.378944Z  INFO mini_redis: Request took 5ms
1
```

##### ping

用法
```
ping [message]
```

若连接成功，且没有指定输出内容，则输出 "pong"，若连接已经失效，则直接报 error
```s
mini-redis>  ping    
2023-09-11T16:45:51.690397Z  INFO mini_redis: Request took 1ms
pong
```

若指定输出内容，则会把输出内容输出
```s
mini-redis>  ping 123
2023-09-12T13:12:47.359970Z  INFO mini_redis: Request took 1ms
123
```

##### subscribe

开启此命令后会进入监听 channel的状态，除非主动 ctrl-c ，不然程序会一直监听，语法如下
```
subscribe <channal_name>
```

进入监听后会进入如下状态，等待 publish
```s
mini-redis>  subscribe 456
The message is as follow: 

```

##### publish

publish 指令格式如下
```
publish <channel_name> <message>
```

当 publish 后会返回收到信息的客户端的个数
```s
mini-redis>  publish 456 shabi
2023-09-11T16:49:44.928856Z  INFO mini_redis: Request took 2ms
publish success. The number of subscriber is 1
```

而监听端也会收到相应信息
```s
mini-redis>  subscribe 456
The message is as follow: 
2023-09-11T16:49:44.929214Z  INFO mini_redis: Request took 121340ms
shabi

```

若没有 subscriber 时则会输出以下信息
```s
mini-redis>  publish 456 7
2023-09-11T16:50:48.907189Z  INFO mini_redis: Request took 1ms
No subscriber found
```

##### multi

> 目前只支持直连主节点

multi 指令格式如下：

``` shell
multi
```

主服务端会返回一个 `txn_id` 作为事务的标识，所有之后发出的基础任务都会被压入任务队列，直到 exec 执行。

##### exec

> 目前只支持直连主节点

exec 指令格式如下：

``` shell
exec
```

主服务端会根据客户端发送的 `txn_id` 对事务对应的任务队列逐一弹出执行。有关 [watch](#watch) 的说明如下一条目所示。

##### watch

> 目前只支持直连主节点

watch 指令格式如下：

``` shell
watch <key>
```

在 watch 某一键值时，在下一个事务发出 exec 时，如果 watch 与 exec 之间该键值被**除自己之外**的客户端修改，则 exec 执行失败。

##### exit

输入该指令退客户端

#### 中间件使用 - 敏感词过滤

```s
mini-redis>  set 123 傻逼
2023-09-14T15:57:22.291364Z ERROR client: Application(ApplicationError { kind: ApplicationErrorKind(0), message: "命令中有敏感词'傻逼'" })
mini-redis>  get 123
2023-09-14T15:57:23.728263Z  INFO mini_redis: Request took 4ms
(nil)
```

# miniredis

## 编译运行方法

需要分别编译proxy节点的工程文件`redis_proxy/`和redis节点的工程文件`mini_redis/`。

编译proxy节点工程文件

```shell
cd redis_proxy/
cargo update
cargo build
```

编译redis节点的工程文件

```shell
cd mini-redis/
cargo update
cargo build
```

编辑配置文件`redis.conf`，配置文件格式如下所示

```shell
# 定义redis集群的ip地址和端口，每行代表一个主从redis架构，多行表示整个集群有多个主从架构，每行中第一个为主节点，后续节点为从节点
[Server]
[::]:45000, [::]:45001, [::]:45002
[::]:46000, [::]:46001, [::]:46002
# 定义了proxy的ip和端口
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

使用redis节点的工程文件`mini_redis/`中带有的client即可进行访问

```shell
cd mini-redis/
cargo run --bin client 127.0.0.1:41000
```

运行时携带了命令行参数，其为要连接的服务端的ip地址。运行后服务端会进入一个交互端口，会出现以下提示符，直接在其后输入命令即可
```
mini-redis> 
```

## get指令

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

不存在则会返回`(nil)`表示空

```s
mini-redis>  get 3
2023-09-11T16:40:45.471091Z  INFO mini_redis: Request took 2ms
(nil)
```

## set指令

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

## del指令

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

## ping

用法
```
ping [message]
```

若连接成功，且没有指定输出内容，则输出"pong"，若连接已经失效，则直接报error
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



## subscribe

开启此命令后会进入监听channel的状态，除非主动ctrl-c，不然程序会一直监听，语法如下
```
subscribe <channal_name>
```

进入监听后会进入如下状态，等待publish
```s
mini-redis>  subscribe 456
The message is as follow: 

```

## publish指令

publish指令格式如下
```
publish <channel_name> <message>
```

当publish后会返回收到信息的客户端的个数
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

若没有subscriber时则会输出以下信息
```s
mini-redis>  publish 456 7
2023-09-11T16:50:48.907189Z  INFO mini_redis: Request took 1ms
No subscriber found
```

## exit

输入该指令退客户端

## 敏感词过滤

```s
mini-redis>  set 123 傻逼
2023-09-14T15:57:22.291364Z ERROR client: Application(ApplicationError { kind: ApplicationErrorKind(0), message: "命令中有敏感词'傻逼'" })
mini-redis>  get 123
2023-09-14T15:57:23.728263Z  INFO mini_redis: Request took 4ms
(nil)
```

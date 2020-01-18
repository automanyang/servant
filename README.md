## Servant——分布式开发的利器

Servant是一个rpc框架库，是一个全部用rust实现的rpc框架，使用简单，方便扩展。

### 例子，hello_servant
千言万语莫如代码举例，就用最简单的hello_servant举例。

1. 定义客户端与服务的接口，一般是在lib类型的crate中定义：
```rust
// -- lib.rs --
#[servant::invoke_interface]
pub trait Hello {
    fn hello(&self, n: i32) -> String;
}
```

2. 在服务端实现接口，在一个网络地址向客户提供服务，一般是在bin类型crate中：
```rust
// -- server.rs --

use {
    hello_servant::{Hello, HelloServant},
    servant::{Context, Server},
    std::sync::{Arc, Mutex},
    async_std::task,
};

// --

struct HelloEntity;
impl Hello for HelloEntity {
    fn hello(&self, _ctx: Option<Context>, n: i32) -> String {
        dbg!(n);
        format!("Hello {}. Welcome to Servant world.", n)
    }
}

// 使用了async_std库的main attributes，方便async/await调用
#[async_std::main]
async fn main() {
    // 生成一个Server，其中的()模板参赛是Notifier的占位参赛，因为这个例子中没有使用Notifier
    let s = Server::<()>::new();

    // 生成HelloServant对象，并加入register中
    let h = HelloEntity;
    let name = "h1";
    s.servant_register().add_servant(
        HelloServant::<HelloEntity>::category(),
        Arc::new(Mutex::new(HelloServant::new(name, h))));

    // 打开端口，向客户提供服务
    let addr = "127.0.0.1:1188";
    let r = task::block_on(s.accept_on(addr));
    assert_eq!(true, r.is_ok());
}
```

这就是服务端的全部代码，其中的_ctx参数，我们暂且不用关心。

3. 在客户端请求服务，一般也是在的bin类型crate中：
```rust
// -- client.rs --

use {
    servant::{Client, Context},
    hello_servant::{HelloProxy},
};

// --

#[async_std::main]
async fn main() {
    // 生成Client对象，并连接到Server等待的地址
    let c = Client::new();
    let addr = "127.0.0.1:1188".to_string();
    let terminal = match c.connect_to(addr).await {
        Ok(t) => t,
        Err(e) => {
            dbg!(e);
            return;
        }
    };

    // 要提前知道请求服务对象的唯一性名称，这里的"h1"与服务端要一致
    let name = "h1";

    // 生成proxy对象，并请求服务
    let mut h = HelloProxy::new(Context::new(), name, &terminal);
    let msg = h.hello(8).await;
    dbg!(&msg);
}
```

这是客户端的全部代码。

4. 分别在不同的终端中，先执行server，在执行client，client的输出如下：
```bash
xt@sf315:~/dev/rust/async/target/debug$ ./hello_client
[hello-servant/src/client.rs:29] &msg = Ok(
    "Hello 8. Welcome to Servant world.",
)
```

server端也会输出：
```bash
xt@sf315:~/dev/rust/async/target/debug$ ./hello_server
[hello-servant/src/server.rs:15] n = 8
```

这就是全部，真的很简单明了。当然，更深入的内容还需要探索。

### 术语

1. Oid：object id，是由对象名称和对象类型组成的一个唯一标识。

2. Interface：接口定义，客户端的都是按照接口向服务端发起请求。

3. Entity：在服务端实现接口的对象实体，必须注册到ServantRegister中，才可以被客户端调用。

4. Proxy：Servant对象在客户端的代理，客户端通过Proxy发出请求调用。

5. Terminal：代表了客户端的网络层，所有Proxy都是基于Terminal创建的。客户端连接到服务端后，就会产生一个Terminal。

6. Adapter：代表了服务端的网络层，服务端在接收到客户端的网络连接请求后，创建一个Adapter与客户端进行网络通信。Terminal与Adapter是一对一的关系。

### 接口类型

1. invoke接口：接口中的方法可以有返回值，客户端发出请求后，调用线程被阻塞直到服务端答复请求或超时，服务端使用返回值答复客户端的请求。

2. watch接口：就像一个特殊的invoke接口，每个服务端只能有一个实现watch接口的对象。此接口中定义服务端基本的信息，客户端可以获取这些基本信息。相比与其他invoke接口，此接口的实现对象没有Oid，所以客户端可以不需要Oid就发出请求。

3. report接口：接口中的方法没有返回值，客户端发出请求后不会阻塞，服务端也不会对请求做任何答复。本接口的数据流方向只能是从客户端到服务端。

4. notify接口：与report接口类似，接口中的方法也没有返回值，每个服务端只能有一个notifier，向所有的客户端发送通知。在客户端每个Terminal可以绑定一个实现notify接口的对象，用于接受和处理从Adapter来的通知信息。本接口的数据流方向是从服务端到客户端。

### 对象类型

有三种对象类型：全局对象、用户对象和临时对象。

1. 全局对象：就是在服务端始终存在的对象，全局可见，随时为客户端提供服务。

2. 用户对象：这是用户创建的对象，只有本用户可见。

3. 临时对象：用户可创建临时对象，该用户退出后，临时对象被删除。
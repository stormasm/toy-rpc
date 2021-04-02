# toy-rpc

## A async RPC crate that mimics the `golang`'s `net/rpc` package and supports both `async-std` and `tokio`.

This crate aims at providing an easy-to-use RPC that is similar to `golang`'s
`net/rpc`.

The usage is similar to that of `golang`'s `net/rpc` with functions sharing similar
names and functionalities. Certain function names are changed to be more "rusty".
Because `rust` doesn't have reflection, attribute macros are used to make certain
method "exported".

### Content

- [Breaking Changes](#breaking-changes)
- [Crate Feature Flags](#crate-feature-flags)
  - [Default Features](#default-features)
- [Documentation](#documentation)
- [Examples](#examples)
  - [Example service definition](#example-service-definition)
  - [RPC over TCP with `async-std`](#rpc-over-tcp-with-async-std)
  - [RPC over TCP with `tokio`](#rpc-over-tcp-with-tokio)
  - [HTTP integration with `tide`](#http-integration-with-tide)
  - [HTTP integration with `actix-web`](#http-integration-with-actix-web)
  - [HTTP integration with `warp`](#http-integration-with-warp)
  - [RPC client for HTTP](#rpc-client-for-http)
- [Change Log](#change-log)
- [Future Plan](#future-plan)

- [Re-exports](#reexports)
- [Modules](#modules)

### Breaking Changes

The most recent breaking changes will be reflected here.

#### Version 0.6.0-alpha

- In short, this update makes the crate resemble closer to the usage of `go`'s `net/rpc` package
- Service registration is simplified to `Server::builder().register(foo_service).build()`. The examples will be
updated accordingly. Thus
    - `service!()` macro will be deprecated
    - `register` function now takes only one argument, which is the instance of the service
    - on the client side, the service name will just be the name of the struct. for example,
        to call a RPC method on `struct Foo { }` service, the client simply uses
        `.async_call("Foo.<method>").await` where `<method>` should be replaced with the RPC method
    - you can still register multiple services on the same server. However, only one object of the same type
        can be registered on the same server. Multiple servers are needed to have multiple objects of the same type.

### Crate Feature Flags

The feature flags can be put into two categories.


Choice of serialization/deserialzation

- `serde_bincode`: the default codec will use `bincode`
for serialization/deserialization
- `serde_json`: the default codec will use `serde_json`
for `json` serialization/deserialization
- `serde_cbor`: the default codec will use `serde_cbor`
for serialization/deserialization
- `serde_rmp`: the default codec will use `rmp-serde`
for serialization/deserialization

Choice of runtime

- `async_std_runtime`: supports usage with `async-std`
- `tokio_runtime`: supports usage with `tokio`
- `http_tide`: enables `tide` integration on the server side. This also enables `async_std_runtime`
- `http_actix_web`: enables `actix-web` integration on the server side. This also enables `tokio_runtime`
- `http_warp`: enables integration with `warp` on the server side. This also enables `tokio_runtime`

Other trivial feature flags are listed below, and they are likely of no actual usage for you.

- `docs`
- `std`: `serde/std`. There is no actual usage right now.

#### Default Features

```toml
[features]
default = [
    "serde_bincode",
    "async_std_runtime"
]
```

### Documentation

The following documentation is adapted based on `golang`'s documentation.

This crate provides access to the methods marked with `#[export_impl]`
and `#[export_method]` of an object across a network connection. A server
registers an object, making it visible as a service with a name provided by the user.
After the registration, the "exported" methods will be accessible remotely.
A server may register multiple objects as multiple services, and multiple
objects of **different** types could be registered on the same
`Server` object. Only one object(service) of the same type can be registered on
one server; multiple servers are needed for multiple objects of the same type.

To export a method, use `#[export_method]` attribute in an impl block marked with
`#[export_impl]` attribute. This crate currently `only` support using `#[export_impl]` attribute
on `one` impl block per type.

```rust
struct ExampleService { }

#[export_impl]
impl ExampleService {
    #[export_method]
    async fn exported_method(&self, args: ()) -> Result<String, String> {
        Ok("This is an exported method".to_string())
    }

    async fn not_exported_method(&self, args: ()) -> Result<String, String> {
        Ok("This method is NOT exported".to_string())
    }
}
```

The methods to export must meet the following criteria on the server side

- the method resides in an impl block marked with `#[export_impl]`
- the method is marked with `#[export_method]` attribute
- the method takes one argument other than `&self` and returns a `Result<T, E>`

  - the argument must implement trait `serde::Deserialize`
  - the `Ok` type `T` of the result must implement trait `serde::Serialize`
  - the `Err` type `E` of the result must implement trait `ToString`

- the method is essentially in the form

```rust
struct ServiceState { }

#[export_impl]
impl ServiceState {
    #[export_method]
    async fn method_name(&self, args: Req) -> Result<Res, ErrorMsg>
    where
        Req: serde::Deserialize,
        Res: serde::Serialize,
        ErrorMsg: ToString,
    {
        unimplemented!()
    }
}
```

`Req` and `Res` are marshaled/unmarshaled (serialized/deserialized) by `serde`.
Realistically the `Req` and `Res` type must also be marshaled/unmarshaled on
the client side, and thus `Req` and `Res` must both implement *both*
`serde::Serialize` and `serde::Deserialize`.

The method's argument reprements the argument provided by the client caller,
and the `Ok` type of result represents success parameters to be returned to
the client caller. The `Err` type of result is passed back to the client as
a `String`.

The server may handle requests on a single connection by calling `serve_conn`,
and it may handle multiple connections by creating a `async_std::net::TcpListener`
and call `accept`. Integration with HTTP currently only supports `tide` by calling
`into_endpoint`.

A client wishing to use the service establishes a `async_std::net::TcpStream` connection
and then creates `Client` over the connection. The convenience function `dial` performs
this step for raw TCP socket connection, and `dial_http` performs this for an HTTP
connection. A `Client` with HTTP connection or socket connection has three methods, `call`, `async_call`,
and `spawn_task`, to specify the service and method to call and the argument. Please note that
the service and method name is case sensitive, and following Rust's naming convention,
the service name should be in CamelCase, for example, if a service is defined as `pub struct Foo {}`,
the client needs to use `async_call("Foo.echo").await` to make the remote call.

- `call` method is synchronous and waits for the remote call
to complete and then returns the result in a blocking manner.
- `async_call` is the `async` versions of `call` and `call_http`,
respectively. Because they are `async` functions, they must be called with `.await` to
be executed.
- `spawn_task` method spawns an `async` task and returns a `JoinHandle`.
The result can be obtained using the `JoinHandle`. Please note that
`async_std::task::JoinHandle` and `tokio::task::JoinHandle` behave slightly different.
Executing `.await` on `async_std::task::JoinHandle` returns `Result<Res, toy_rpc::error::Error>`.
However, executing `.await` on `tokio::task::JoinHandle` returns
`Result<Result<Res, toy_rpc::error::Error>, tokio::task::JoinError>.
- A client stub trait is generated automatically which allows usage such as `client.foo().echo("data").await`
where `foo()` represents a call to the `Foo{}` service while `echo()` represents the RPC method
for the `Foo{}` service. More details can be found [below](#client-stub)

Unless an explicity codec is set up (with `serve_codec` method, HTTP is *NOT* supported yet),
the default codec specified by one of the following features tags (`serde_bincode`, `serde_json`
`serde_cbor`, `serde_rmp`) will be used to transport data.

### `async-std` and `tokio`

Starting from version `0.5.0-beta.2`, you can use `toy-rpc` with either runtime by choosing
the corresponding feature flag (`async_std_runtime`, `tokio_runtime`).

### HTTP integrations

Similar to choosing the runtimes, `toy-rpc` supports integration with `actix-web`, `tide`,
and `warp` by choosing the corresponding feature flag (`http_tide`, `http_actix_web`
`http_warp`). Starting from version `0.5.0-beta.0` the integration is implemented using
WebSocket as the transport protocol, and the `DEFAULT_RPC_SERVER=_rpc_` is appended to the path you
supply to the HTTP framework. The client side support is not based on `async_tungstenite`
and removed usage of `surf`. Thus versions >=`0.5.0-beta.0` are **NOT** compatible
with versions <`0.5.0-beta.0`. The [examples](#examples) below are also updated to reflect
the changes.

### Client Stub

The `#[export_impl]` macro now also generates client stubs that internally uses `async_call`.
For example, if the `Example {}` service is registered on the server as `"example_service"`.
If you want to call the `echo(&self, arg: u32)` RPC method on the `Example {}` service, you
can conveniently use `client.example().echo(3).await.unwrap()`. The generated stub follows the
snake case, for example
    - if a service is defined as `pub struct Foo {}`, the generated stub will be `foo()`
    - if a service is defined as `pub struct FooBar {}`, the generated stub will be `foo_bar()`
    - if a service is defined asx `pub struct FooBarService {}`, the generated stub will be `foo_bar_service()`

```rust

pub mod rpc {
    use toy_rpc::macros::export_impl;
    use serde::{Serialize, Deserialze};

    pub struct Example { }

    #[export_impl]
    impl Example {
        pub async fn echo(&self, arg: u32) -> Result<u32, String> {
            Ok(arg)
        }
    }
}

// import everything from the `rpc` mod to include generated client stub
use rpc::*;

#[async_std::main]
async fn main() {
    let addr = "127.0.0.1:23333";
    let client = Client::dial(addr).await.unwrap();

    // assume the service is registered as "example" on the server side
    let reply = client.example().echo(3).await.unwrap();
    println!("Reply: {}", reply);
}

```


### Examples

A few simple examples are shown below. More examples can be found in the `examples`
directory in the repo. All examples here will assume the follwing
[RPC service definition](#example-service-definition) below.

The examples here will also need some **other** dependencies

```toml
[dependencies]
# you may need to change feature flags for different examples
toy-rpc = { version = "0.6.0-alpha" }

# optional depending on the choice of runtime or http framework for different examples
async-std = { version = "1.9.0", features = ["attributes"] }
tokio = { version = "1.2.0", features = ["rt", "rt-multi-thread", "macros", "net", "sync"] }
tide = "0.16.0"
actix-web = "3.3.2"
warp = "0.3.0"

# other dependencies needed for the examples here
async-trait = "0.1.42"
env_logger = "0.8.2"
log = "0.4.14"
serde = { version = "1.0.123", features = ["derive"] }

```

#### Example Service Definition

```rust
pub mod rpc {
    use serde::{Serialize, Deserialize};
    use toy_rpc::macros::export_impl;

    // use tokio::sync::Mutex; // uncomment this for the examples that use tokio runtime
    // use async_std::sync::Mutex; // uncomment this for the examples that use async-std runtime

    pub struct ExampleService {
        pub counter: Mutex<i32>
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ExampleRequest {
        pub a: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ExampleResponse {
        a: u32,
    }

    #[async_trait::async_trait]
    trait Rpc {
        async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String>;
    }

    #[async_trait::async_trait]
    #[export_impl]
    impl Rpc for ExampleService {
        #[export_method]
        async fn echo(&self, req: ExampleRequest) -> Result<ExampleResponse, String> {
            let mut counter = self.counter.lock().await;
            *counter += 1;

            let res = ExampleResponse{ a: req.a };
            Ok(res)
        }
    }
}
```

#### RPC over TCP with `async-std`

This example will assume the [RPC service defined above](#example-service-definition),
and you may need to uncomment the line `use async_std::sync::Mutex;` in the RPC service definition
for this example.

The default feature flags will work with the example below.

server.rs

```rust
use async_std::net::TcpListener;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use toy_rpc::macros::service;
use toy_rpc::Server;

use crate::rpc; // assume the rpc module can be found here

#[async_std::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    // notice that the`service!()` macro is no longer needed
    let server = Server::builder()
        .register(example_service)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Starting listener at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });
    handle.await;
}
```

client.rs

```rust
use toy_rpc::Client;
use toy_rpc::error::Error;

// import everything from the `rpc` mod to include generated client stub
use crate::rpc::*; // assume the rpc module can be found here

#[async_std::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let client = Client::dial(addr).await.unwrap();

    let args = ExampleRequest{a: 1};

    // Use synchronous call
    let reply: Result<rpc::ExampleResponse, Error> = client.call("Example.echo", &args);
    println!("{:?}", reply);

    // or use the generated client stub
    let reply = client.example().echo(&args).await;
    println!("{:?}", reply);
    client.close().await;
}
```

#### RPC over TCP with `tokio`

This example will assume the [RPC service defined above](#example-service-definition)
and you may need to uncomment the line `use tokio::sync::Mutex;` in the RPC service definition
for this example.

The default feature flags will **NOT** work for this example, and you need to change
the feature flags.

```rust
[dependencies]
toy_rpc = { version = "0.6.0-alpha", default-features = false, features = ["serde_bincode", "tokio_runtime"] }
```

server.rs

```rust
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task;
use toy_rpc::macros::service;
use toy_rpc::Server;

use crate::rpc; // assume the rpc module can be found here

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    // notice that the second argument in `service!()` macro is a path
    let server = Server::builder()
        .register(example_service)
        .build();

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Starting listener at {}", &addr);

    let handle = task::spawn(async move {
        server.accept(listener).await.unwrap();
    });

    // tokio JoinHandle returns an extra result
    handle.await.unwrap();
}
```

client.rs

```rust
use toy_rpc::Client;
use toy_rpc::error::Error;

// import everything from the `rpc` mod to include generated client stub
use crate::rpc::*; // assume the rpc module can be found here

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let client = Client::dial(addr).await.unwrap();

    let args = rpc::ExampleRequest{a: 1};

    // Use synchronous call
    let reply: Result<rpc::ExampleResponse, Error> = client.call("Example.echo", &args);
    println!("{:?}", reply);

    // or use the generated client stub
    let reply = client.example().echo(&args).await;
    println!("{:?}", reply);
    client.close().await;

    client.close().await;
}
```


#### HTTP integration with `tide`

This example will assume the [RPC service defined above](#example-service-definition)
and you may need to uncomment the line `use async_std::sync::Mutex;` in the RPC service definition
for this example.

An example client to use with HTTP can be found in a separate example [here](#rpc-client-for-http).
The default feature flags will **NOT** work with this example, and you need to change
the feature flags.

```toml
toy_rpc = { version = "0.6.0-alpha", default-features = false, features = ["serde_bincode", "http_tide"] }
```

server.rs

```rust
use async_std::sync::{Arc, Mutex};
use toy_rpc::macros::service;
use toy_rpc::Server;

use crate::rpc; // assume the rpc module can be found here

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register(example_service)
        .build();

    let mut app = tide::new();
    app.at("/rpc/").nest(server.handle_http());
    // with `http_tide`, the line above can also be replaced with the line below
    //app.at("/rpc/").nest(server.into_endpoint());

    app.listen(addr).await?;
    Ok(())
}

```

#### HTTP integration with `actix-web`

This example will assume the [RPC service defined above](#example-service-definition)
and you may need to uncomment the line `use tokio::sync::Mutex;` in the RPC service definition
for this example.

An example client to use with HTTP can be found in a another example [here](#rpc-client-for-http).
The default feature flags will **NOT** work with this example, and you need to change
the feature flags.

```toml
toy_rpc = { version = "0.6.0-alpha", default-features = false, features = ["serde_bincode", "http_actix_web"] }
```

server.rs

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use actix_web::{App, HttpServer, web};
use toy_rpc::macros::service;
use toy_rpc::Server;

use crate::rpc; // assume the rpc module can be found here

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:8080";
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register(example_service)
        .build();

    let app_data = web::Data::new(server);

    HttpServer::new(
        move || {
            App::new()
                .service(
                    web::scope("/rpc/")
                        .app_data(app_data.clone())
                        .configure(Server::handle_http())
                        // with `http_actix_web`, the line above can also be replaced with the line below
                        //.configure(Server::scope_config)
                )
        }
    )
    .bind(addr)?
    .run()
    .await
}

```

#### HTTP integration with `warp`

This example will assume the [RPC service defined above](#example-service-definition)
and you may need to uncomment the line `use tokio::sync::Mutex;` in the RPC service definition
for this example.

An example client to use with HTTP can be found in a another example [here](#rpc-client-for-http).
The default feature flags will **NOT** work with this example, and you need to change
the feature flags.

```toml
toy_rpc = { version = "0.6.0-alpha", default-features = false, features = ["serde_bincode", "http_warp"] }
```

server.rs

```rust
use warp::Filter;
use std::sync::Arc;
use tokio::sync::Mutex;
use toy_rpc::macros::service;
use toy_rpc::Server;

use crate::rpc; // assume the rpc module can be found here

#[tokio::main]
async fn main() {
    env_logger::init();
    let example_service = Arc::new(
        rpc::ExampleService {
            counter: Mutex::new(0),
        }
    );

    let server = Server::builder()
        .register(example_service)
        .build();

    let routes = warp::path("rpc")
        .and(server.handle_http());

    // RPC will be served at "ws://127.0.0.1:8080/rpc/_rpc_"
    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}

```

#### RPC client for HTTP

This example will assume the [RPC service defined above](#example-service-definition).
The default feature flags will work with this example. However, you may also use
client with any runtime or http feature flag.

All HTTP examples assumes that the RPC server is found at "127.0.0.1/rpc/" endpoint.

```rust
use toy_rpc::Client;
use toy_rpc::error::Error;

use crate::rpc; // assume the rpc module can be found here

// choose the runtime attribute accordingly
//#[tokio::main]
#[async_std::main]
async fn main() {
    // note that the url scheme is "ws"
    let addr = "ws://127.0.0.1:8080/rpc/";
    let client = Client::dial_http(addr).await.unwrap();

    let args = rpc::ExampleRequest{a: 1};
    // Use synchronous call
    let reply: Result<rpc::ExampleResponse, Error> = client.call("Example.echo", &args);
    println!("{:?}", reply);

    // or use the generated client stub
    let reply = client.example().echo(&args).await;
    println!("{:?}", reply);

    client.close().await;
}
```

### Change Log

#### Version 0.6.0-alpha

- In short, this update makes the crate resemble closer to the usage of `go`'s `net/rpc` package
- Service registration is simplified to `Server::builder().register(foo_service).build()`. The examples will be
updated accordingly. Thus
    - `service!()` macro will be deprecated
    - `register` function now takes only one argument, which is the instance of the service
    - on the client side, the service name will just be the name of the struct. for example,
        to call a RPC method on `struct Foo { }` service, the client simply uses
        `.async_call("Foo.<method>").await` where `<method>` should be replaced with the RPC method
    - you can still register multiple services on the same server. However, only one object of the same type
        can be registered on the same server. Multiple servers are needed to have multiple objects of the same type.

#### 0.5.4

- Handlers are now stored as a `fn` pointer as opposed to a trait object.

#### 0.5.3

- The `#[export_impl]` macro now generates client stub functions by generating a new trait for `toy_rpc::Client`.

#### 0.5.0

Breaking changes

- HTTP integration is now accomplished using WebSocket with `async_tungstenite`, and thus HTTP connections
of versions <0.5.0 are not compatible with versions >=0.5.0.
- The custom binary transport protocol now includes a magic byte at the beginning, making
versions <0.5.0 **NOT** compatible with versions >= 0.5.0;
- `toy_rpc::error::Error` changed from struct-like variants to simple enum variants
- Changes to feature flags
    - "logging" feature flag is removed
    - "surf" feature flag is removed
    - "tide" is changed to "http_tide"
    - "actix-web" is changed to "http_actix_web"
    - added "http_warp" feature flag
    - added "async_std_runtime"
    - added "tokio_runtime"

Non-breaking changes

- Removed `Stream` and `Sink` impl from the custom binary transport protocol `Frame`

#### 0.4.5

- Added `Sink` implementation for the custom binary transport protocol `Frame`

#### 0.4.4

- Modified traits `CodecRead`, `CodecWrite`, `ServerCodec`, `ClientCodec` to no longer
return number of bytes written
- The number of bytes written for header and body will be logged separately

#### 0.4.3

- Removed previously unused NoneError
- Unified `call`, `async_call` and `spawn_task` for socket client
and HTTP client. The `call_http`, `async_call_http`, and `spawn_task_http`
methods are kept for compatibility.

#### 0.4.2

- Temporary fix of `spawn_task()` and `spawn_task_http()` with `Arc<Mutex<_>>` until
lifetime with async task is figured out. As a result, `Client` no longer needs to be declared `mut`.

#### 0.4.1

- Updated documentation

#### 0.4.0

- Added `actix-web` feature flag to support integration with `actix-web`

#### 0.3.1

- Added `serde_rmp` features flag
- Updated and corrected examples in the documentation

#### 0.3.0

- Added `serde_cbor` feature flag
- Changed `bincode` feature flag to `serde_bincode`


### Future Plan

The following items are in no particular order.

- improve error handling
- improve logging message
- support other I/O connection
- more tests


License: MIT/Apache-2.0
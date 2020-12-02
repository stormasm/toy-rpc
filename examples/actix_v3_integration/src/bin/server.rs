use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use async_trait::async_trait;
use async_std::sync::{Arc, Mutex};

use toy_rpc::macros::{export_impl, service};
use toy_rpc::Server;

use actix_v3_integration::rpc::{Rpc, BarService, FooRequest, FooResponse};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hello")
}

pub struct FooService {
    counter: Mutex<u32>,
}

#[async_trait]
#[export_impl]
impl Rpc for FooService {
    #[export_method]
    async fn echo(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse { a: req.a, b: req.b };

        Ok(res)
        // Err("echo error".into())
    }

    #[export_method]
    async fn increment_a(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a + 1,
            b: req.b,
        };

        Ok(res)
        // Err("increment_a error".into())
    }

    #[export_method]
    async fn increment_b(&self, req: FooRequest) -> Result<FooResponse, String> {
        let mut counter = self.counter.lock().await;
        *counter += 1;

        let res = FooResponse {
            a: req.a,
            b: req.b + 1,
        };

        Ok(res)
        // Err("increment_b error".into())
    }

    #[export_method]
    async fn get_counter(&self, _: ()) -> Result<u32, String> {
        let counter = self.counter.lock().await;
        let res = *counter;
        Ok(res)
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:23333";

    // let app = App::new().service(
    //     server.into_handler()
    // );

    // let mut app = App::new();

    let foo_service = Arc::new(FooService {
        counter: Mutex::new(0),
    });
    let bar_service = Arc::new(BarService {});

    let server = Server::builder()
        .register("foo_service", service!(foo_service, FooService))
        .register("bar_service", service!(bar_service, actix_v3_integration::rpc::BarService))
        .build();

    let app_data = web::Data::new(server);

    HttpServer::new(
        move || {
            println!("HttpServer::new");

            App::new()
                .service(hello)
                .service(
                    web::scope("/rpc/")
                        .app_data(app_data.clone())
                        .service(Server::into_actix_scope())
                )
        }
    )
    .bind(addr)?
    .run()
    .await
}
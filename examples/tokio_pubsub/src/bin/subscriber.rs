use toy_rpc::Client;
use futures::StreamExt;

use tokio_pubsub::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut client = Client::dial(ADDR).await.unwrap();
    let mut count_sub = client.subscriber::<Count>(10).unwrap();
    while let Some(item) = count_sub.next().await {
        let item = item.unwrap();
        println!("{}", item);
    }
}
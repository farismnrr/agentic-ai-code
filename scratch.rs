use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = "142.250.191.46:80".parse().unwrap(); // google IP
    let client = reqwest::Client::builder()
        .resolve("example.com", addr)
        .build()
        .unwrap();
    
    let res = client.get("https://example.com").send().await;
    println!("{:?}", res);
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::limited(10)).build().unwrap();
    match client.get("http://127.0.0.1:42693/redirect-to-loopback-port").send().await {
        Ok(res) => println!("OK: {}", res.status()),
        Err(e) => println!("Error: {:?}", e),
    }
}

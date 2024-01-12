use futures_util::StreamExt;
use serde_json::Value;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
	let address = "wss://fstream.binance.com/ws/btcusdt@markPrice";
	let url = url::Url::parse(address).unwrap();
	let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
	println!(" ++ Connected ++ ");
	let (_, read) = ws_stream.split();
	let ws_to_stdout = {
		read.for_each(|message| async {
			let data = message.unwrap().into_data();
			match serde_json::from_slice::<Value>(&data) {
				Ok(json) => {
					if let Some(price_str) = json.get("p") {
						let price: f32 = price_str.as_str().unwrap().parse().unwrap();
						println!("{}", price);
					}
				}
				Err(e) => {
					println!("Failed to parse message as JSON: {}", e);
				}
			}
		})
	};
	ws_to_stdout.await;
}

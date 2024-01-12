use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
	let address = "wss://fstream.binance.com/ws/btcusdt@markPrice";
	let url = url::Url::parse(address).unwrap();
	let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
	println!("Connected");
	let (_, read) = ws_stream.split();
	let ws_to_stdout = {
		read.for_each(|message| async {
			let data = message.unwrap().into_data();
			tokio::io::stdout().write_all(&data).await.unwrap();
		})
	};
	ws_to_stdout.await;
}

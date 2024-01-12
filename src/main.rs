use std::sync::{Arc, Mutex};

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

	let main_line = Arc::new(Mutex::new(MainLine::default()));
	let ws_to_stdout = {
		read.for_each(|message| async {
			let data = message.unwrap().into_data();
			match serde_json::from_slice::<Value>(&data) {
				Ok(json) => {
					if let Some(price_str) = json.get("p") {
						let price: f32 = price_str.as_str().unwrap().parse().unwrap();
						let main_line_str: String;
						{
							let mut main_line = main_line.lock().unwrap();
							main_line.btcusdt = Some(price);
							main_line_str = main_line.display();
						}
						println!("{}", main_line_str);
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

#[derive(Default)]
struct MainLine {
	pub btcusdt: Option<f32>,
	pub percent_longs: Option<f32>,
}
impl MainLine {
	pub fn display(&self) -> String {
		let btcusdt_display = match self.btcusdt {
			Some(value) => format!("{:.0}", value),
			None => "None".to_string(),
		};

		let percent_longs_display = match self.percent_longs {
			Some(value) => format!("|{:.2}", value),
			None => "".to_string(),
		};

		format!("{}{}", btcusdt_display, percent_longs_display)
	}
}

//```python
//def get_percent_longs(symbol="btc", type="global"):
//	symbol = symbol.upper() + "USDT"
//	type = ("global", "Account") if type == "global" else ("top", "Position")
//	try:
//		r = requests.get(f"https://fapi.binance.com/futures/data/{type[0]}LongShort{type[1]}Ratio?symbol={symbol}&period=5m&limit=1").json()
//		longs = float(r[0]["longAccount"])
//		longs = str(round(longs, 2))
//		longs = longs[1:]
//		if len(longs) == 2:
//			longs += "0"
//
//		return longs
//	except Exception as e:
//		print(f"Error getting LSR: {e}")
//		return None
//```

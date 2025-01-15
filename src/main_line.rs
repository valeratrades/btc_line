use std::sync::{Arc, Mutex};

use color_eyre::eyre::{bail, eyre, Result};
use futures_util::StreamExt;
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tracing::debug;


use crate::{config::AppConfig, output::Output};

#[derive(Default, Debug)]
pub struct MainLine {
	pub btcusdt: Option<f64>,
	pub percent_longs: Option<f64>,
}
impl MainLine {
	pub fn display(&self, config: &AppConfig) -> String {
		let price_line = self.btcusdt.map_or("None".to_string(), |v| format!("{:.0}", v));
		let mut longs_line = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", v));

		if config.label {
			longs_line = format!("L/S:{}", longs_line);
		}

		format!("{}|{}", price_line, longs_line)
	}

	pub async fn websocket(self_arc: Arc<Mutex<Self>>, config: AppConfig, output: Arc<Mutex<Output>>) {
		loop {
			let handle = binance_websocket_listen(self_arc.clone(), &config, output.clone());

			handle.await;
			{
				let mut lock = self_arc.lock().unwrap();
				lock.btcusdt = None;
			}
			debug!("Restarting Binance Websocket in 30 seconds...");
			tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
		}
	}

	pub async fn collect(self_arc: Arc<Mutex<MainLine>>) {
		let percent_longs_handler = get_percent_longs("BTCUSDT", PercentLongsScope::Global);

		let percent_longs: Option<f64> = match percent_longs_handler.await {
			Ok(percent_longs) => Some(percent_longs as f64),
			Err(e) => {
				debug!("Failed to get LSR: {}", e);
				None
			}
		};

		let mut self_lock = self_arc.lock().unwrap();
		self_lock.percent_longs = percent_longs;
	}
}

async fn binance_websocket_listen(self_arc: Arc<Mutex<MainLine>>, config: &AppConfig, output: Arc<Mutex<Output>>) {
	let address = "wss://fstream.binance.com/ws/btcusdt@markPrice";
	let (ws_stream, _) = connect_async(address).await.expect("Failed to connect");
	let (_, read) = ws_stream.split();

	read.for_each(|message| {
		let main_line = self_arc.clone(); // Cloning the Arc for each iteration
		let output = output.clone(); // Can i get rid of these?
		async move {
			let data = message.unwrap().into_data();
			match serde_json::from_slice::<Value>(&data) {
				Ok(json) =>
					if let Some(price_str) = json.get("p") {
						let price: f64 = price_str.as_str().unwrap().parse().unwrap();
						let mut main_line = main_line.lock().unwrap();
						main_line.btcusdt = Some(price);
						let mut output_lock = output.lock().unwrap();
						output_lock.main_line_str = main_line.display(config);
						output_lock.out().unwrap();
					},
				Err(e) => {
					debug!("Failed to parse message as JSON: {}", e);
				}
			}
		}
	})
	.await;
}

#[allow(dead_code)]
enum PercentLongsScope {
	Global,
	Top,
}
impl PercentLongsScope {
	fn request_url_insertions_tuple(&self) -> (String, String) {
		match self {
			PercentLongsScope::Global => ("global".to_string(), "Account".to_string()),
			PercentLongsScope::Top => ("top".to_string(), "Position".to_string()),
		}
	}
}
async fn get_percent_longs(symbol_str: &str, type_: PercentLongsScope) -> Result<f64> {
	let mut symbol = symbol_str.to_uppercase();
	if !symbol.contains("USDT") {
		symbol = format!("{}USDT", symbol);
	}

	let (ins_0, ins_1) = type_.request_url_insertions_tuple();

	let url = format!("https://fapi.binance.com/futures/data/{}LongShort{}Ratio?symbol={}&period=5m&limit=1", ins_0, ins_1, symbol);

	let resp = reqwest::get(&url).await?;
	let json: Vec<Value> = resp.json().await?;
	if let Some(long_account_str) = json.get(0).and_then(|item| item["longAccount"].as_str()) {
		long_account_str.parse::<f64>().map_err(|e| eyre!("Failed to parse 'longAccount' as f64: {}", e))
	} else {
		bail!("'longAccount' field missing or not a string in response: {:?}", json)
	}
}

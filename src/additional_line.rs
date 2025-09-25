use std::{
	fs::{self, File},
	io::{BufRead, BufReader},
	path::Path,
	sync::{Arc, Mutex},
};

use color_eyre::eyre::{bail, Result};
use serde::Deserialize;
use tracing::debug;
use v_utils::NowThen;

use crate::config::AppConfig;

//TODO!: implement tiny graphics
#[derive(Default, Debug)]
pub struct AdditionalLine {
	open_interest_change: Option<NowThen>,
	btc_volume_change: Option<NowThen>,
	pub enabled: bool,
}

impl AdditionalLine {
	pub fn display(&self, config: &AppConfig) -> String {
		if !self.enabled {
			return "".to_string();
		}

		let mut oi_str = self.open_interest_change.as_ref().map_or("None".to_string(), |v| format!("{}", v));
		let mut v_str = self.btc_volume_change.as_ref().map_or("None".to_string(), |v| format!("{}", v));

		if config.label {
			oi_str = format!("OI:{oi_str}");
			v_str = format!("V:{}", v_str);
		}
		format!("{} {}", oi_str, v_str)
	}

	pub async fn collect(self_arc: Arc<Mutex<Self>>, config: &AppConfig) {
		let comparison_offset_h = config.comparison_offset_h;

		let client = reqwest::Client::new();
		let open_interest_change_handler = get_open_interest_change(&client, "BTCUSDT", comparison_offset_h);
		let btc_volume_change_handler = get_btc_volume_change(&client, comparison_offset_h);

		self_arc.lock().unwrap().open_interest_change = match open_interest_change_handler.await {
			Ok(open_interest_change) => Some(open_interest_change),
			Err(e) => {
				debug!("Failed to get Open Interest: {}", e);
				None
			}
		};
		self_arc.lock().unwrap().btc_volume_change = match btc_volume_change_handler.await {
			Ok(btc_volume_change) => Some(btc_volume_change),
			Err(e) => {
				debug!("Failed to get BTC Volume: {}", e);
				None
			}
		};
	}

	pub async fn listen_to_pipe(self_arc: Arc<Mutex<Self>>, config: AppConfig, output: Arc<Mutex<crate::output::Output>>) {
		let pipe_path = "/tmp/btc_line/toggle_additional";

		// Create /tmp/btc_line directory if it doesn't exist
		let pipe_dir = "/tmp/btc_line";
		if !Path::new(pipe_dir).exists() {
			let _ = fs::create_dir_all(pipe_dir);
		}

		// Create named pipe if it doesn't exist
		if !Path::new(pipe_path).exists() {
			let _ = std::process::Command::new("mkfifo").arg(pipe_path).status();
		}

		loop {
			// Attempt to open the named pipe; this will block until the other side is opened for writing
			if let Ok(file) = File::open(pipe_path) {
				let reader = BufReader::new(file);
				reader.lines().for_each(|line| {
					if let Ok(line) = line {
						if let Ok(arg) = line.parse::<bool>() {
							if self_arc.lock().unwrap().enabled != arg {
								self_arc.lock().unwrap().enabled = arg;
								let mut output_lock = output.lock().unwrap();
								output_lock.additional_line_str = self_arc.lock().unwrap().display(&config);
								output_lock.out().unwrap();
							}
						}
					}
				});
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(125)).await;
		}
	}
}

/// Compares btc OI today against 24h ago (changes based on `comparison_offset_h`)
async fn get_open_interest_change(client: &reqwest::Client, symbol: &str, comparison_offset_h: usize) -> Result<NowThen> {
	let url = format!(
		"https://fapi.binance.com/futures/data/openInterestHist?symbol={}&period=5m&limit={}",
		symbol,
		comparison_offset_h * 12 + 1
	);

	let response = client.get(&url).send().await?;
	if response.status().is_success() {
		let json_string = response.text().await?;
		let r: Vec<OpenInterestHist> = serde_json::from_str(&json_string)?;

		let now: f64 = r[0].sumOpenInterestValue.parse()?;
		let then: f64 = r[r.len() - 1].sumOpenInterestValue.parse()?;

		Ok(NowThen::new(now, then))
	} else {
		bail!("Failed to get Open Interest Change: {}", response.status())
	}
}

/// Compares two last periods of `comparison_offset_h` hours. Default is yesterday against the day before.
async fn get_btc_volume_change(client: &reqwest::Client, comparison_offset_h: usize) -> Result<NowThen> {
	let interval = comparison_offset_h * 12;
	let url = format!("https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=5m&limit={}", interval + interval);

	let response = client.get(&url).send().await?;
	if response.status().is_success() {
		let json_string = response.text().await?;
		let r: Vec<Kline> = serde_json::from_str(&json_string)?;

		let split = r.split_at(interval);
		let now: f64 = split.0.iter().map(|v| v.quote_asset_volume.parse::<f64>().unwrap()).sum();
		let then: f64 = split.1.iter().map(|v| v.quote_asset_volume.parse::<f64>().unwrap()).sum();

		Ok(NowThen::new(now, then))
	} else {
		bail!("Failed to get BTC Volume: {}", response.status())
	}
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct OpenInterestHist {
	symbol: String,
	sumOpenInterest: String,
	sumOpenInterestValue: String,
	timestamp: i64,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
struct Kline {
	open_time: i64,
	open: String,
	high: String,
	low: String,
	close: String,
	volume: String,
	close_time: i64,
	quote_asset_volume: String,
	number_of_trades: usize,
	taker_buy_base_asset_volume: String,
	taker_buy_quote_asset_volume: String,
	ignore: String,
}

use std::sync::{Arc, Mutex};

use tracing::debug;
use v_exchanges::prelude::*;
use v_utils::Percent;

use crate::{config::AppConfig, output::Output};

#[derive(Default, Debug)]
pub struct MainLine {
	pub btcusdt: Option<f64>,
	pub percent_longs: Option<Percent>,
}
impl MainLine {
	pub fn display(&self, config: &AppConfig) -> String {
		let price_line = self.btcusdt.map_or("None".to_string(), |v| format!("{v:.0}"));
		let mut longs_line = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", *v));

		if config.label {
			longs_line = format!("L/S:{longs_line}");
		}

		format!("{price_line}|{longs_line}")
	}

	pub async fn websocket(self_arc: Arc<Mutex<Self>>, config: AppConfig, output: Arc<Mutex<Output>>, exchange: Arc<Binance>) {
		async fn binance_websocket_listen(self_arc: Arc<Mutex<MainLine>>, config: &AppConfig, output: Arc<Mutex<Output>>, exchange: Arc<Binance>) {
			let mut connection = exchange.ws_trades(vec![("BTC", "USDT").into()], Instrument::Perp).unwrap();
			while let Ok(trade_event) = connection.next().await {
				let price = trade_event.price;
				let main_line_str = {
					let mut self_lock = self_arc.lock().unwrap();
					self_lock.btcusdt = Some(price);
					self_lock.display(config)
				};
				let output_copy = {
					let mut output_lock = output.lock().unwrap();
					output_lock.main_line_str = main_line_str;
					output_lock.clone()
				};
				output_copy.out().await.unwrap();
			}
		}
		loop {
			let handle = binance_websocket_listen(self_arc.clone(), &config, output.clone(), Arc::clone(&exchange));

			handle.await;
			{
				let mut lock = self_arc.lock().unwrap();
				lock.btcusdt = None;
			}
			debug!("Restarting Binance Websocket in 30 seconds...");
			tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
		}
	}

	pub async fn collect(self_arc: Arc<Mutex<MainLine>>, bn: Arc<Binance>) {
		let lsr_handler = bn.lsr(("BTC", "USDT").into(), "5m".into(), 1.into(), v_exchanges::binance::data::LsrWho::Global);
		let percent_longs: Option<Percent> = match lsr_handler.await {
			Ok(percent_longs) => Some(*percent_longs[0]),
			Err(e) => {
				tracing::warn!("Failed to get LSR: {}", e);
				None
			}
		};

		let mut self_lock = self_arc.lock().unwrap();
		self_lock.percent_longs = percent_longs;
	}
}

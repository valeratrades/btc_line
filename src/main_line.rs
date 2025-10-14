use std::{rc::Rc, sync::Arc, time::Duration};

use tokio::time::Interval;
use v_exchanges::{ExchangeResult, prelude::*};
use v_utils::{Percent, trades::Pair};

use crate::config::{Settings, SettingsError};

#[derive(Debug)]
pub struct MainLine {
	settings: Rc<Settings>,

	btcusdt_price: Option<f64>,
	percent_longs: Option<Percent>,

	lsr_interval: Interval,

	ws_connection: Box<dyn ExchangeStream<Item = Trade>>,
	binance_agent: Arc<Binance>,
}
impl MainLine {
	pub fn try_new(settings: Rc<Settings>, bn: Arc<Binance>, lsr_update_freq: Duration) -> ExchangeResult<Self> {
		let pairs: Vec<Pair> = vec![("BTC", "USDT").into()];
		let instrument = Instrument::Perp;
		let ws_connection = bn.ws_trades(pairs, instrument)?;

		let lsr_interval = tokio::time::interval(lsr_update_freq);

		Ok(Self {
			settings,
			ws_connection,
			binance_agent: bn,
			lsr_interval,
			// defaults {{{
			btcusdt_price: None,
			percent_longs: None,
			//,}}}
		})
	}

	/// # Returns
	/// if any of the data has been updated, returns `true`
	pub async fn collect(&mut self) -> ExchangeResult<bool> {
		let handle_lsr = async {
			let lsr_result = self
				.binance_agent
				.lsr(("BTC", "USDT").into(), "5m".into(), 1.into(), v_exchanges::binance::data::LsrWho::Global)
				.await;

			let percent_longs: Option<Percent> = match lsr_result {
				Ok(percent_longs) => Some(*percent_longs[0]),
				Err(e) => {
					tracing::warn!("Failed to get LSR: {e}");
					None
				}
			};

			if self.percent_longs != percent_longs {
				self.percent_longs = percent_longs;
				true
			} else {
				false
			}
		};

		let handle_trade = async {
			match self.ws_connection.next().await {
				Ok(trade) => {
					let new_price = Some(trade.price);
					assert_ne!(trade.price, 0.); //dbg: had it print `&main_line.display()? = "0|0.63"` few times, not sure why
					if self.btcusdt_price != new_price {
						self.btcusdt_price = new_price;
						true
					} else {
						false
					}
				}
				Err(e) => {
					tracing::warn!("Failed to get trade: {e}");
					false
				}
			}
		};

		let changed = tokio::select! {
			_ = self.lsr_interval.tick() => handle_lsr.await,
			changed = handle_trade => changed,
		};

		Ok(changed)
	}

	pub fn display(&self) -> Result<String, SettingsError> {
		let price = self.btcusdt_price.map_or("None".to_string(), |v| format!("{v:.0}"));
		let mut lsr = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", *v));

		if self.settings.config()?.label {
			lsr = format!("L/S:{lsr}");
		}

		let s = format!("{price}|{lsr}");
		Ok(s)
	}
}

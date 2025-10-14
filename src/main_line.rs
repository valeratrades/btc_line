use std::{
	sync::Arc,
	time::{Duration, SystemTime},
};

use v_exchanges::{ExchangeResult, adapters::generics::ws::WsError, prelude::*};
use v_utils::{Percent, trades::Pair};

use crate::config::{AppConfig, Settings, SettingsError};

#[derive(Debug)]
pub struct MainLine {
	settings: Arc<Settings>,

	btcusdt_price: Option<f64>,
	percent_longs: Option<Percent>,

	lsr_frequency: Duration,
	lsr_last_pull: SystemTime,

	ws_connection: Box<dyn ExchangeStream<Item = Trade>>,
	binance_agent: Arc<Binance>,
}
impl MainLine {
	pub fn try_new(settings: Arc<Settings>, bn: Arc<Binance>) -> ExchangeResult<Self> {
		let pairs: Vec<Pair> = vec![("BTC", "USDT").into()];
		let instrument = Instrument::Perp;
		let ws_connection = bn.ws_trades(pairs, instrument)?;

		Ok(Self {
			settings,
			ws_connection,
			binance_agent: bn,
			// defaults {{{
			btcusdt_price: None,
			percent_longs: None,
			lsr_frequency: Duration::default(),
			lsr_last_pull: std::time::UNIX_EPOCH,
			//,}}}
		})
	}

	/// # Returns
	/// if any of the data has been updated, returns `true`
	pub async fn collect(&mut self) -> ExchangeResult<bool> {
		let lsr_handler = self.binance_agent.lsr(("BTC", "USDT").into(), "5m".into(), 1.into(), v_exchanges::binance::data::LsrWho::Global);

		let mut changed = false;

		let percent_longs: Option<Percent> = match lsr_handler.await {
			Ok(percent_longs) => Some(*percent_longs[0]),
			Err(e) => {
				tracing::warn!("Failed to get LSR: {e}");
				None
			}
		};

		//DO: loop select

		if self.percent_longs != percent_longs {
			self.percent_longs = percent_longs;
			changed = true;
		}

		match changed {
			true => Ok(true),
			false => Ok(false),
		}
	}

	pub fn display(&self) -> Result<String, SettingsError> {
		let price_line = self.btcusdt_price.map_or("None".to_string(), |v| format!("{v:.0}"));
		let mut longs_line = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", *v));

		if self.settings.config()?.label {
			longs_line = format!("L/S:{longs_line}");
		}

		let s = format!("{price_line}|{longs_line}");
		Ok(s)
	}

	/// returns a closure that would request Lsrs from exchange, if enough time has elapsed since last req
	async fn lsr_pull(&self) -> impl Fn() -> ExchangeResult<Lsrs> + Send {
		tokio::time::sleep_until(deadline).await;
		#[rustfmt::skip]
		async move || {
        self.binance_agent.lsr(("BTC", "USDT").into(), "5m".into(), 1.into(), v_exchanges::binance::data::LsrWho::Global)
    }
	}

	async fn trade_ws(&mut self) -> Result<Trade, WsError> {
		(*self.ws_connection).next().await
	}
}

use std::{pin::pin, rc::Rc, sync::Arc, time::Duration};

use futures::future::{Either, select};
use tokio::time::Interval;
use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};
use v_utils::{Percent, trades::Pair};

use crate::config::LiveSettings;

#[derive(Debug)]
pub struct MainLine {
	settings: Rc<LiveSettings>,

	btcusdt_price: Option<f64>,
	percent_longs: Option<Percent>,

	lsr_interval: Interval,

	ws_connection: Box<dyn ExchangeStream<Item = Trade>>,
	binance_agent: Arc<Binance>,
}
impl MainLine {
	pub fn try_new(settings: Rc<LiveSettings>, bn: Arc<Binance>, lsr_update_freq: Duration) -> ExchangeResult<Self> {
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
		enum Event {
			Tick,
			Trade(Result<Trade, WsError>),
		}

		let event = {
			let tick_fut = pin!(self.lsr_interval.tick());
			let trade_fut = pin!(self.ws_connection.next());

			match select(tick_fut, trade_fut).await {
				Either::Left((_tick, _trade_fut)) => Event::Tick,
				Either::Right((trade_result, _tick_fut)) => Event::Trade(trade_result),
			}
		}; // futures dropped here, borrows released

		let changed = match event {
			Event::Tick => self.handle_lsr().await,
			Event::Trade(trade_result) => self.handle_trade(trade_result),
		};

		Ok(changed)
	}

	async fn handle_lsr(&mut self) -> bool {
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
	}

	fn handle_trade(&mut self, trade_result: Result<Trade, WsError>) -> bool {
		match trade_result {
			Ok(trade) =>
				if self.btcusdt_price != Some(trade.price) {
					self.btcusdt_price = Some(trade.price);
					true
				} else {
					false
				},
			Err(e) => {
				tracing::warn!("Failed to get trade: {e}");
				false
			}
		}
	}

	pub fn display(&self) -> String {
		let price = self.btcusdt_price.map_or("None".to_string(), |v| format!("{v:.0}"));
		let mut lsr = self.percent_longs.map_or("".to_string(), |v| format!("{:.2}", *v));

		if self.settings.config().label {
			lsr = format!("L/S:{lsr}");
		}

		let s = format!("{price}|{lsr}");
		s
	}
}

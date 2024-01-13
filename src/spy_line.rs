use crate::config::Config;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::connect_async;

#[derive(Default, Debug)]
pub struct SpyLine {
	pub spy_price: Option<f32>,
	//TODO!: have another loop that updates spy_price to None if last timestamp is more than 60s old.
	last_message_timestamp: DateTime<Utc>,
}

impl SpyLine {
	pub fn display(&self) -> String {
		self.spy_price.map_or_else(|| "".to_string(), |v| format!("{:.2}", v))
	}

	pub async fn websocket(config: Config, self_arc: Arc<Mutex<Self>>) {
		let alpaca_key = &config.spy.alpaca_key;
		let alpaca_secret = &config.spy.alpaca_secret;
		loop {
			let handle = spy_websocket_listen(self_arc.clone(), alpaca_key, alpaca_secret);

			handle.await;
			{
				let mut lock = self_arc.lock().unwrap();
				lock.spy_price = None;
			}
			eprintln!("Restarting Spy Websocket in 30 seconds...");
			tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
		}
	}
}

async fn spy_websocket_listen(self_arc: Arc<Mutex<SpyLine>>, alpaca_key: &str, alpaca_secret: &str) {
	//TODO!!!!!!!!!: correct connection from set keys

	//             extra_headers={'Content-Type': 'application/msgpack'},
	//
	//async def _auth(self):
	//   await self._ws.send(
	//       msgpack.packb({
	//           'action': 'auth',
	//           'key':    self._key_id,
	//           'secret': self._secret_key,
	//       }))
	//   r = await self._ws.recv()
	//   msg = msgpack.unpackb(r)
	//   if msg[0]['T'] == 'error':
	//       raise ValueError(msg[0].get('msg', 'auth failed'))
	//   if msg[0]['T'] != 'success' or msg[0]['msg'] != 'authenticated':
	//       raise ValueError('failed to authenticate')

	// docs https://alpaca.markets/deprecated/docs/api-documentation/api-v2/market-data/alpaca-data-api-v1/streaming/

	let endpoint = "wss://data.alpaca.markets/stream";

	let authenticaton_response = reqwest::Client::new()
		.get(endpoint)
		.header("Content-Type", "application/msgpack")
		.header("APCA-API-KEY-ID", alpaca_key)
		.header("APCA-API-SECRET-KEY", alpaca_secret)
		.send()
		.await
		.expect("Failed to authenticate with Alpaca");

	dbg!(&authenticaton_response);

	//	// authenticate
	//	{
	//    "action": "authenticate",
	//    "data": {
	//        "key_id": "<YOUR_KEY_ID>",
	//        "secret_key": "<YOUR_SECRET_KEY>"
	//    }
	//}

	// listen
	//	{
	//    "action": "listen",
	//    "data": {
	//        "streams": ["T.SPY", "Q.SPY", "AM.SPY"]
	//    }
	//}

	let url = url::Url::parse(endpoint).unwrap();
	let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
	let (_, read) = ws_stream.split();

	read.for_each(|message| {
		let spy_line = self_arc.clone(); // Cloning the Arc for each iteration
		async move {
			let data = message.unwrap().into_data();
			match serde_json::from_slice::<Value>(&data) {
				Ok(json) => {
					println!("{:?}", json)
				}
				Err(e) => {
					println!("Failed to parse message as JSON: {}", e);
				}
			}
		}
	})
	.await;
}

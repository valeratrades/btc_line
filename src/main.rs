use std::{pin::Pin, sync::Arc, time::Duration};

use btc_line::{
	additional_line::AdditionalLine,
	config::{self, LiveSettings},
	main_line::MainLine,
	output::{FlushFut, LineName, Output},
};
use clap::Parser;
use color_eyre::eyre::Result;
use futures_util::{StreamExt as _, stream::FuturesUnordered};
use v_exchanges::{
	Exchange,
	adapters::{binance::BinanceOption, generics::ws::WsConfig},
	binance::Binance,
};
use v_utils::utils::exit_on_error;

#[derive(Parser)]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
struct Cli {
	#[clap(flatten)]
	settings_flags: config::SettingsFlags,
	#[command(subcommand)]
	command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
	/// Manage configuration: write defaults, diff against defaults, and generate the JSON Schema / Nix module for editor type-awareness.
	Config {
		#[command(subcommand)]
		cmd: config::SettingsCommand,
	},
}

#[tokio::main]
async fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();
	if let Some(Command::Config { cmd }) = cli.command {
		// Never returns — performs the requested config operation and exits.
		config::AppConfig::handle_settings_command(cmd, cli.settings_flags);
	}
	let settings = exit_on_error(LiveSettings::new(cli.settings_flags, Duration::from_secs(5)));
	let eyre_result = start(settings).await;
	exit_on_error(eyre_result);
}

enum LineInstance {
	Main(Box<MainLine>),
	Additional(AdditionalLine),
}

async fn start(settings: LiveSettings) -> Result<()> {
	let settings = Arc::new(settings);
	let mut output = Output::new(Arc::clone(&settings));
	// A silent-but-open trade socket is caught in `message_timeout + response_timeout` (a probe Ping
	// followed by a missing Pong → reconnect). Match the v_exchanges defaults explicitly to pin the
	// price line's staleness ceiling here, where the exchange client is built.
	let mut bn = Binance::default();
	let mut ws_config = WsConfig::default();
	ws_config.set_message_timeout(Duration::from_secs(32)).expect("non-zero literal");
	ws_config.set_response_timout(Duration::from_secs(8)).expect("non-zero literal");
	bn.update_default_option(BinanceOption::WsConfig(ws_config));

	let main_line = MainLine::new(Arc::clone(&settings), bn, Duration::from_secs(15));
	let additional_line = AdditionalLine::new(Arc::clone(&settings), Arc::new(Binance::default()) as Arc<dyn Exchange>, Duration::from_secs(15));

	type BoxFut = Pin<Box<dyn std::future::Future<Output = (LineName, LineInstance, v_exchanges::ExchangeResult<bool>)>>>;
	let mut futures: FuturesUnordered<BoxFut> = FuturesUnordered::new();

	futures.push(Box::pin(async move {
		let mut ml = main_line;
		let result = ml.collect().await;
		(LineName::Main, LineInstance::Main(Box::new(ml)), result)
	}));

	futures.push(Box::pin(async move {
		let mut al = additional_line;
		let result = al.collect().await;
		(LineName::Additional, LineInstance::Additional(al), result)
	}));

	// Single slot for the deferred eww-rate-limit flush future. We drive it inline (no `tokio::spawn`); the inner `flush_scheduled` flag guarantees at most one is ever in flight, so a single slot is sufficient.
	// If a previous flush hasn't finished we drop the new one — the still-running flush will pick up any newly-queued `pending` values.
	let mut pending_flush: Option<FlushFut> = None;

	//LOOP: main loop
	loop {
		tokio::select! {
			Some((line_name, instance, result)) = futures.next() => {
				let changed = result?;
				if changed {
					let display_str = match &instance {
						LineInstance::Main(ml) => ml.display(),
						LineInstance::Additional(al) => al.display(),
					};
					if let Some(flush_fut) = output.output(line_name, display_str).await?
						&& pending_flush.is_none()
					{
						pending_flush = Some(flush_fut);
					}
				}

				match instance {
					LineInstance::Main(ml) => {
						futures.push(Box::pin(async move {
							let mut ml = ml;
							let result = ml.collect().await;
							(LineName::Main, LineInstance::Main(ml), result)
						}));
					}
					LineInstance::Additional(al) => {
						futures.push(Box::pin(async move {
							let mut al = al;
							let result = al.collect().await;
							(LineName::Additional, LineInstance::Additional(al), result)
						}));
					}
				}
			}
			() = async {
				match pending_flush.as_mut() {
					Some(f) => f.as_mut().await,
					None => std::future::pending().await,
				}
			} => {
				pending_flush = None;
			}
		}
	}
}

mod additional_line;
pub mod config;
mod main_line;
pub mod output;

use std::{pin::Pin, sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::eyre::Result;
use futures_util::{StreamExt as _, stream::FuturesUnordered};
use output::Output;
use v_exchanges::{Exchange, binance::Binance};
use v_utils::utils::exit_on_error;

use crate::{
	additional_line::AdditionalLine,
	config::LiveSettings,
	main_line::MainLine,
	output::{FlushFut, LineName},
};

#[derive(Parser)]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
struct Cli {
	#[clap(flatten)]
	settings_flags: config::SettingsFlags,
}

#[tokio::main]
async fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();
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
	let main_line = MainLine::try_new(Arc::clone(&settings), Binance::default(), Duration::from_secs(15)).await?;
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
	// If a previous flush hasn't finished we drop the new one — the still-running flush will pick up any newly-stashed `pending_value`.
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

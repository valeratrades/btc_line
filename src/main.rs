mod additional_line;
pub mod config;
mod main_line;
pub mod output;
use std::{pin::Pin, rc::Rc, sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::eyre::Result;
use futures_util::{StreamExt as _, stream::FuturesUnordered};
use output::Output;
use v_exchanges::{Exchange, binance::Binance};
use v_utils::utils::exit_on_error;

use crate::{additional_line::AdditionalLine, config::LiveSettings, main_line::MainLine, output::LineName};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[clap(flatten)]
	settings_flags: config::SettingsFlags,
}

#[tokio::main]
async fn main() {
	v_utils::clientside!(".log");
	let cli = Cli::parse();
	let settings = match LiveSettings::new(cli.settings_flags, Duration::from_secs(5)) {
		Ok(s) => s,
		Err(e) => {
			eprintln!("Failed to initialize settings: {e}");
			std::process::exit(1);
		}
	};

	//TODO!!!!!!!!: specify set of errors on which we just wait 30s and retry
	let eyre_result = start(settings).await;
	exit_on_error(eyre_result);
}

enum LineInstance {
	Main(MainLine),
	Additional(AdditionalLine),
}

//Q: should this return ExchangeResult, or actually just wrap over infinite retries?
async fn start(settings: LiveSettings) -> Result<()> {
	let settings = Rc::new(settings);
	let mut output = Output::new(Rc::clone(&settings));
	let mut bn = Binance::default();
	bn.set_max_tries(3);
	let bn_arc = Arc::new(bn);

	let main_line = MainLine::try_new(Rc::clone(&settings), Arc::clone(&bn_arc), Duration::from_secs(15))?;
	let additional_line = AdditionalLine::new(Rc::clone(&settings), bn_arc as Arc<dyn Exchange>, Duration::from_secs(15));

	type BoxFut = Pin<Box<dyn std::future::Future<Output = (LineName, LineInstance, v_exchanges::ExchangeResult<bool>)>>>;
	let mut futures: FuturesUnordered<BoxFut> = FuturesUnordered::new();

	futures.push(Box::pin(async move {
		let mut ml = main_line;
		let result = ml.collect().await;
		(LineName::Main, LineInstance::Main(ml), result)
	}));

	futures.push(Box::pin(async move {
		let mut al = additional_line;
		let result = al.collect().await;
		(LineName::Additional, LineInstance::Additional(al), result)
	}));

	while let Some((line_name, instance, result)) = futures.next().await {
		let changed = result?;
		if changed {
			let display_str = match &instance {
				LineInstance::Main(ml) => ml.display(),
				LineInstance::Additional(al) => al.display(),
			};
			output.output(line_name, display_str).await?;
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

	Ok(())
}

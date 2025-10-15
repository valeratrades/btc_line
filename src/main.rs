mod additional_line;
pub mod config;
mod main_line;
pub mod output;
use std::{cell::RefCell, pin::Pin, rc::Rc, sync::Arc, time::Duration};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::Result;
use futures_util::{StreamExt as _, stream::FuturesUnordered};
use output::Output;
use v_exchanges::{ExchangeName, binance::Binance};
use v_utils::{io::ExpandedPath, utils::exit_on_error};

use crate::{additional_line::AdditionalLine, config::Settings, main_line::MainLine, output::LineName};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
	#[arg(long, default_value = "~/.config/btc_line.toml")]
	config: ExpandedPath,
}

#[derive(Subcommand)]
enum Commands {
	/// Start the program
	Start(NoArgs),
}
#[derive(Args)]
struct NoArgs {}

#[tokio::main]
async fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();
	let settings = Settings::new(cli.config.0, Duration::from_secs(5));

	match cli.command {
		Commands::Start(_) => {
			//TODO!!!!!!!!: specify set of errors on which we just wait 30s and retry
			let eyre_result = start(settings).await;
			exit_on_error(eyre_result);
		}
	}
}

//Q: should this return ExchangeResult, or actually just wrap over infinite retries?
async fn start(settings: Settings) -> Result<()> {
	let settings = Rc::new(settings);
	let mut output = Output::new(Rc::clone(&settings));
	let bn = Arc::new(Binance::default());

	let mut binance_exchange = ExchangeName::Binance.init_client();
	binance_exchange.set_max_tries(3);

	let main_line = Rc::new(RefCell::new(MainLine::try_new(Rc::clone(&settings), Arc::clone(&bn), Duration::from_secs(15))?));
	let additional_line = Rc::new(RefCell::new(AdditionalLine::new(Rc::clone(&settings), Arc::from(binance_exchange), Duration::from_secs(15))));

	type BoxFut = Pin<Box<dyn std::future::Future<Output = (LineName, v_exchanges::ExchangeResult<bool>)>>>;
	let mut futures: FuturesUnordered<BoxFut> = FuturesUnordered::new();

	{
		let ml = Rc::clone(&main_line);
		futures.push(Box::pin(async move {
			let result = ml.borrow_mut().collect().await;
			(LineName::Main, result)
		}));
	}
	{
		let al = Rc::clone(&additional_line);
		futures.push(Box::pin(async move {
			let result = al.borrow_mut().collect().await;
			(LineName::Additional, result)
		}));
	}

	while let Some((line_name, result)) = futures.next().await {
		let changed = result?;
		if changed {
			let display_str = match line_name {
				LineName::Main => main_line.borrow().display()?,
				LineName::Additional => additional_line.borrow().display()?,
				_ => unreachable!(),
			};
			output.output(line_name, display_str).await?;
		}

		match line_name {
			LineName::Main => {
				let ml = Rc::clone(&main_line);
				futures.push(Box::pin(async move {
					let result = ml.borrow_mut().collect().await;
					(LineName::Main, result)
				}));
			}
			LineName::Additional => {
				let al = Rc::clone(&additional_line);
				futures.push(Box::pin(async move {
					let result = al.borrow_mut().collect().await;
					(LineName::Additional, result)
				}));
			}
			_ => unreachable!(),
		}
	}

	Ok(())
}

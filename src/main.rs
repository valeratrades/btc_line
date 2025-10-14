mod additional_line;
pub mod config;
mod main_line;
pub mod output;
use std::{rc::Rc, sync::Arc, time::Duration};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::Result;
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

	let mut main_line = MainLine::try_new(Rc::clone(&settings), Arc::clone(&bn), Duration::from_secs(15))?;
	let mut additional_line = AdditionalLine::new(Rc::clone(&settings), Arc::new(binance_exchange), Duration::from_secs(10)); //dbg: should be like 60s

	loop {
		tokio::select! {
			result = main_line.collect() => {
				let main_line_changed = result?;
				if main_line_changed {
					output.output(LineName::Main, main_line.display()?).await?;
					dbg!(&main_line.display()?);
				}
			},
			result = additional_line.collect() => {
				let additional_line_changed = result?;
				if additional_line_changed {
					output.output(LineName::Additional, additional_line.display()?).await?;
					dbg!(&additional_line.display()?);
				}
			},
		}
	}
}

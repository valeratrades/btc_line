mod additional_line;
pub mod config;
mod main_line;
pub mod output;
use std::{rc::Rc, sync::Arc, time::Duration};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{Report, Result};
use output::Output;
use v_exchanges::{ExchangeResult, binance::Binance};
use v_utils::{io::ExpandedPath, utils::exit_on_error};

use crate::{config::Settings, main_line::MainLine, output::LineName};

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

	let mut main_line = MainLine::try_new(Rc::clone(&settings), Arc::clone(&bn), Duration::from_secs(15))?;
	//let additional_line = AdditionalLine::new(settings, bn);

	//dbg
	loop {
		let main_line_updated = main_line.collect().await?;
		if main_line_updated {
			output.output(LineName::Main, main_line.display()?).await?;
		}
	}

	//let mut cycle = 0;
	//loop {
	//	{
	//		let main_line_str = { main_line.lock().unwrap().display(&config) };
	//		let additional_line_str = { additional_line.lock().unwrap().display(&config) };
	//		let mut output_lock = output.lock().unwrap();
	//		output_lock.main_line_str = main_line_str;
	//		output_lock.additional_line_str = additional_line_str;
	//		output_lock.out().await.unwrap();
	//	}
	//
	//	cycle += 1;
	//	if cycle == 15 {
	//		cycle = 1; // rolls to 1, so I can make special cases for 0
	//	}
	//	tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
	//
	//	Ok(())
	//}
}

mod additional_line;
pub mod config;
mod main_line;
pub mod output;
use std::{
	rc::Rc,
	sync::{Arc, Mutex},
	time::Duration,
};

use clap::{Args, Parser, Subcommand};
use output::Output;
use v_exchanges::binance::Binance;
use v_utils::io::ExpandedPath;

use crate::config::Settings;

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
	let settings = Rc::new(Settings::new(cli.config.0, Duration::from_secs(5)));

	match cli.command {
		Commands::Start(_) => {
			let output = Output::new(Rc::clone(&settings));

			let main_line = Arc::new(Mutex::new(main_line::MainLine::default()));
			//let spy_line = Arc::new(Mutex::new(spy_line::SpyLine::default()));
			let additional_line = Arc::new(Mutex::new(additional_line::AdditionalLine::default()));
			let exchange = Arc::new(Binance::default());

			//TODO!!!: change to [].join() along with main loop. Spawns bad.
			let _ = tokio::spawn(main_line::MainLine::websocket(main_line.clone(), Arc::clone(settings), output.clone(), Arc::clone(&exchange)));
			//let _ = tokio::spawn(spy_line::SpyLine::websocket(spy_line.clone(), Arc::clone(settings), output.clone()));
			let mut cycle = 0;
			loop {
				{
					let main_line_handler = main_line::MainLine::collect(Arc::clone(&main_line), Arc::clone(&exchange));
					let additional_line_handler = additional_line::AdditionalLine::collect(additional_line.clone(), &config);

					let _ = main_line_handler.await;
					let _ = additional_line_handler.await;
				}

				{
					let main_line_str = { main_line.lock().unwrap().display(&config) };
					let additional_line_str = { additional_line.lock().unwrap().display(&config) };
					let mut output_lock = output.lock().unwrap();
					output_lock.main_line_str = main_line_str;
					output_lock.additional_line_str = additional_line_str;
					output_lock.out().await.unwrap();
				}

				cycle += 1;
				if cycle == 15 {
					cycle = 1; // rolls to 1, so I can make special cases for 0
				}
				tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
			}
		}
	}
}

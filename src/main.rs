mod additional_line;
pub mod config;
mod main_line;
pub mod output;
mod spy_line;
pub mod utils;
use std::sync::{Arc, Mutex};

use clap::{Args, Parser, Subcommand};
use config::AppConfig;
use output::Output;
use tracing::error;
use v_exchanges::binance::Binance;
use v_utils::io::ExpandedPath;

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
	utils::init_subscriber(None);
	let cli = Cli::parse();
	let config = match AppConfig::new(cli.config) {
		Ok(cfg) => cfg,
		Err(e) => {
			error!("{:?}", e);
			std::process::exit(1);
		}
	};

	match cli.command {
		Commands::Start(_) => {
			let output = Arc::new(Mutex::new(Output::new(config.clone())));

			let main_line = Arc::new(Mutex::new(main_line::MainLine::default()));
			let spy_line = Arc::new(Mutex::new(spy_line::SpyLine::default()));
			let additional_line = Arc::new(Mutex::new(additional_line::AdditionalLine::default()));
			let exchange = Arc::new(Binance::default());

			//TODO!!!: change to [].join() along with main loop. Spawns bad.
			let _ = tokio::spawn(main_line::MainLine::websocket(main_line.clone(), config.clone(), output.clone(), Arc::clone(&exchange)));
			let _ = tokio::spawn(spy_line::SpyLine::websocket(spy_line.clone(), config.clone(), output.clone()));
			let mut cycle = 0;
			loop {
				{
					let main_line_handler = main_line::MainLine::collect(Arc::clone(&main_line), Arc::clone(&exchange));
					let additional_line_handler = additional_line::AdditionalLine::collect(additional_line.clone(), &config);

					let _ = main_line_handler.await;
					let _ = additional_line_handler.await;
				}

				{
					let mut output_lock = output.lock().unwrap();
					output_lock.main_line_str = main_line.lock().unwrap().display(&config);
					output_lock.additional_line_str = additional_line.lock().unwrap().display(&config);
					output_lock.out().unwrap();
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

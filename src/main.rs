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
	let settings = init_settings_with_retry(cli.settings_flags).await;

	let eyre_result = start(settings).await;
	exit_on_error(eyre_result);
}

/// Extract env var name from error like "Environment variable 'FOO' not found"
fn extract_missing_env_var(err: &str) -> Option<&str> {
	let marker = "Environment variable '";
	let start = err.find(marker)? + marker.len();
	let rest = &err[start..];
	let end = rest.find('\'')?;
	Some(&rest[..end])
}

/// Initialize settings with exponential backoff retry for transient errors.
/// Config errors (missing env vars) fail immediately with helpful suggestions.
/// Transient errors retry with e^i delays: ~1s, ~2.7s, ~7.4s, ~20s, ...
async fn init_settings_with_retry(flags: config::SettingsFlags) -> LiveSettings {
	let mut attempt = 0u32;
	loop {
		match LiveSettings::new(flags.clone(), Duration::from_secs(5)) {
			Ok(s) => return s,
			Err(e) => {
				let err_str = format!("{e:#}");

				// Config errors: missing env vars - fail with guidance
				if let Some(env_var) = extract_missing_env_var(&err_str) {
					eprintln!("Missing environment variable: {env_var}\n");
					eprintln!("Pass secrets as flags:");
					eprintln!("  --spy-alpaca-key <KEY>");
					eprintln!("  --spy-alpaca-secret <SECRET>");
					eprintln!();
					eprintln!("Example:");
					eprintln!("  btc_line --spy-alpaca-key 'PKXXX...' --spy-alpaca-secret 'XXX...'");
					std::process::exit(1);
				}

				// Transient errors: nix not ready, network issues - retry with backoff
				let delay_secs = std::f64::consts::E.powi(attempt as i32);
				let delay = Duration::from_secs_f64(delay_secs);
				eprintln!("Transient error (attempt {}): {err_str}\nRetrying in {delay_secs:.1}s...", attempt + 1,);
				tokio::time::sleep(delay).await;
				attempt += 1;
			}
		}
	}
}

enum LineInstance {
	Main(MainLine),
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
		(LineName::Main, LineInstance::Main(ml), result)
	}));

	futures.push(Box::pin(async move {
		let mut al = additional_line;
		let result = al.collect().await;
		(LineName::Additional, LineInstance::Additional(al), result)
	}));

	// Single slot for the deferred eww-rate-limit flush future. We drive it inline (no
	// `tokio::spawn`); the inner `flush_scheduled` flag guarantees at most one is ever in flight,
	// so a single slot is sufficient. Per the requested rule, if a previous flush hasn't finished
	// we drop the new one — the still-running flush will pick up any newly-stashed `pending_value`.
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

use std::{io::Write, path::Path};

use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// # Panics
pub fn init_subscriber(log_path: Option<Box<Path>>) {
	let setup = |make_writer: Box<dyn Fn() -> Box<dyn Write> + Send + Sync>| {
		//let tokio_console_artifacts_filter = EnvFilter::new("tokio[trace]=off,runtime[trace]=off");
		let formatting_layer = tracing_subscriber::fmt::layer().json().pretty().with_writer(make_writer).with_file(true).with_line_number(true)/*.with_filter(tokio_console_artifacts_filter)*/;

		let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or(tracing_subscriber::EnvFilter::new("info"));
		//let env_filter = env_filter
		//      .add_directive("tokio=off".parse().unwrap())
		//      .add_directive("runtime=off".parse().unwrap());

		let error_layer = ErrorLayer::default();

		tracing_subscriber::registry().with(env_filter).with(formatting_layer).with(error_layer).init();
		//tracing_subscriber::registry()
		//  .with(tracing_subscriber::layer::Layer::and_then(formatting_layer, error_layer).with_filter(env_filter))
		//  .with(console_layer)
		//  .init();
	};

	match log_path {
		Some(path) => {
			let path = path.to_owned();

			// Truncate the file before setting up the logger
			{
				let _ = std::fs::OpenOptions::new()
					.create(true)
					.write(true)
					.truncate(true)
					.open(&path)
					.expect("Failed to truncate log file");
			}

			setup(Box::new(move || {
				let file = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("Failed to open log file");
				Box::new(file) as Box<dyn Write>
			}));
		}
		None => {
			setup(Box::new(|| Box::new(std::io::stdout())));
		}
	};
}

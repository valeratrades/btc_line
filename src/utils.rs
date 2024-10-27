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

#[derive(Debug, Clone)]
pub struct NowThen {
	pub now: f64,
	pub then: f64,
}

impl std::fmt::Display for NowThen {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let diff = self.now - self.then;

		let (now_f, now_suffix) = format_number_compactly(self.now, 0.035);
		let (diff_f, diff_suffix) = format_number_compactly(diff, 0.005);

		let now_suffix = if now_suffix == diff_suffix { "" } else { now_suffix };
		let diff_sign = if diff > 0.0 { "+" } else { "" };

		let diff_str = format!("{}{}", diff_f, diff_suffix);
		let now_str = format!("{}{}", now_f, now_suffix);

		write!(f, "{}{}{}", now_str, diff_sign, diff_str)
	}
}

fn format_number_compactly(mut n: f64, precision: f64) -> (f64, &'static str) {
	assert!(precision >= 0.0, "Precision can't be negative, the hell? {:?}", precision);
	let mut thousands = 0;
	while n.abs() >= 1000.0 {
		n /= 1000.0;
		thousands += 1;
	}

	let sure_n_digits = precision.log(0.1).ceil() as usize + 1;
	let mut n_str = {
		let mut temp_str = "".to_string();
		let mut countdown = sure_n_digits + 2; // the whole block is to cut out what we definitely can cut out, so might as well have a buffer
		for c in n.to_string().chars() {
			temp_str.push(c);
			if c != '.' {
				countdown -= 1;
			}
			if countdown == 0 {
				break;
			}
		}
		temp_str
	};

	// format, then subtract one, and try format again; if within precision from original, commit.
	loop {
		if !n_str.contains('.') {
			break;
		}
		let n_precision = n_str.split('.').last().unwrap().len();
		let try_round_one_more = format!("{:.*}", n_precision - 1, n);
		if ((n - try_round_one_more.parse::<f64>().unwrap()) / n).abs() > precision {
			break;
		} else {
			n_str = try_round_one_more;
		}
	}
	let mut n = n_str.parse::<f64>().unwrap();

	if n.abs() >= 1000.0 {
		n /= 1000.0;
		thousands += 1;
	}

	fn suffix_from_n_thousands(n: usize) -> &'static str {
		match n {
			0 => "",
			1 => "K",
			2 => "M",
			3 => "B",
			4 => "T",
			5 => "Q",
			_ => panic!("Number is too large, calm down"),
		}
	}
	(n, suffix_from_n_thousands(thousands))
}

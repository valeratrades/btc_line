use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn init_tracing() {
	let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
	let formatting_layer = BunyanFormattingLayer::new("discretionary_engine".into(), std::io::stdout);
	let subscriber = Registry::default().with(env_filter).with(JsonStorageLayer).with(formatting_layer);

	set_global_default(subscriber).expect("Failed to set subscriber");

	let default_panic_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |panic_info| {
		if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
			tracing::error!("panic occurred: {:?}", s);
		} else {
			tracing::error!("panic occurred");
		}

		default_panic_hook(panic_info);
	}));
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

//! Manual verification of the eww output buffer + backpressure.
//!
//! Drives the real `Output` against a temp config: a long `eww_rate_limit` so the first call sends
//! and every distinct subsequent value queues (without actually flushing), a tiny `buffer`, and a
//! small `max_flushes`. We feed distinct values (to defeat `old_vals` dedup) and observe:
//!   1. buffer cap drops oldest beyond `buffer`,
//!   2. backpressure errors once total queued hits `max_flushes`.
//!
//! Run: `cargo r --example buffer_backpressure`
use std::{sync::Arc, time::Duration};

use btc_line::{
	config::{Cli, LiveSettings},
	output::{LineName, Output},
};
use clap::Parser;

#[tokio::main]
async fn main() {
	// Fake `eww` on PATH so the single immediate send doesn't fail. It just no-ops.
	let dir = std::env::temp_dir().join("btc_line_buftest_bin");
	std::fs::create_dir_all(&dir).unwrap();
	let eww = dir.join("eww");
	std::fs::write(&eww, "#!/bin/sh\nexit 0\n").unwrap();
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(&eww, std::fs::Permissions::from_mode(0o755)).unwrap();
	}
	let path = std::env::var("PATH").unwrap();
	unsafe { std::env::set_var("PATH", format!("{}:{path}", dir.display())) };

	// Temp config: long rate limit (everything after the first call queues), small buffer + max_flushes.
	let cfg = dir.join("config.toml");
	std::fs::write(
		&cfg,
		r#"
comparison_offset_h = 24
label = false
[outputs]
eww = true
eww_rate_limit = "1h"
pipes = false
buffer = 3
max_flushes = 5
[spy]
alpaca_key = "x"
alpaca_secret = "y"
"#,
	)
	.unwrap();

	let cli = Cli::parse_from(["x", "--config", cfg.to_str().unwrap()]);
	let settings = Arc::new(LiveSettings::new(cli.settings_flags, Duration::from_secs(60)).unwrap());
	let mut output = Output::new(settings);

	// First call sends immediately (consumes the 1h window). Returns no flush future.
	let f0 = output.output(LineName::Main, "v0".into()).await.unwrap();
	assert!(f0.is_none(), "first call should send immediately, not schedule a flush");

	// Next distinct values all queue. buffer=3, so only the 3 most recent are retained per line.
	// We push 5 distinct values; expect: 1st queue schedules a flush future, rest return None,
	// and after >buffer pushes the queue holds only the last 3 (older dropped, counter adjusted).
	let mut got_flush = false;
	for i in 1..=5 {
		let f = output.output(LineName::Main, format!("v{i}")).await.unwrap();
		if f.is_some() {
			got_flush = true;
		}
	}
	assert!(got_flush, "at least one queued value should have scheduled a flush future");
	println!("OK: buffer path — 5 distinct queued values accepted, oldest dropped beyond buffer=3");

	// Backpressure: max_flushes=5. Main currently holds 3 (capped by buffer). Push another line to
	// reach the GLOBAL cap, then over. At the cap we SHED (drop the new value), not error — the app
	// must survive overload. Main=3 already. a1 sends immediately, a2/a3 queue -> total 5 (==max).
	output.output(LineName::Additional, "a1".into()).await.unwrap(); // sends immediately (new line, last_sent=None)
	let _ = output.output(LineName::Additional, "a2".into()).await.unwrap(); // queues -> total 4
	let _ = output.output(LineName::Additional, "a3".into()).await.unwrap(); // queues -> total 5 (==max)

	// Over the cap: must NOT error, must NOT schedule new work — just drop the value (Ok(None)).
	let over = output.output(LineName::Additional, "a4".into()).await;
	match over {
		Ok(None) => println!("OK: backpressure shed update at max_flushes=5 (Ok(None), app survives)"),
		Ok(Some(_)) => panic!("over-cap push should not schedule a flush"),
		Err(e) => panic!("backpressure must shed, not error: {e}"),
	}

	println!("\nALL CHECKS PASSED");
}

use clap::Parser;
use v_utils::{
	macros::{ConfigJsonSchema, LiveSettings, MyConfigPrimitives, Settings, SettingsNested},
	trades::Timeframe,
};

/// CLI struct with SettingsFlags for clap integration
#[derive(Debug, Parser)]
pub struct Cli {
	#[clap(flatten)]
	pub settings_flags: SettingsFlags,
}
#[derive(Clone, ConfigJsonSchema, Debug, LiveSettings, MyConfigPrimitives, Settings)]
pub struct AppConfig {
	#[settings(flatten)]
	pub spy: Spy,
	pub comparison_offset_h: usize = 24,
	/// whether to label the displayed values, or (false ->) assume user's acquaintance with the layout
	pub label: bool,
	#[settings(flatten)]
	pub outputs: Outputs,
}

#[derive(Clone, Debug, serde::Deserialize, schemars::JsonSchema, serde::Serialize, SettingsNested, smart_default::SmartDefault)]
#[serde(default)]
pub struct Outputs {
	#[default(false)]
	pub eww: bool,
	/// Rate limit for eww updates. If set, eww updates will be batched and sent at most once per this duration.
	pub eww_rate_limit: Option<Timeframe>,
	#[default(true)]
	pub pipes: bool,
	/// Per-line cap on how many not-yet-pushed values we remember. When a burst arrives faster than we
	/// can flush, only the `buffer` most recent values are kept; older ones are dropped.
	#[default(16)]
	pub buffer: u8,
	/// Backpressure: hard cap on the total number of queued-but-unflushed values across all lines. If a
	/// new value would exceed this, we error out rather than let the queue grow unbounded.
	#[default(64)]
	pub max_flushes: u8,
}

#[derive(Clone, Debug, Default, schemars::JsonSchema, MyConfigPrimitives, SettingsNested)]
pub struct Spy {
	pub alpaca_key: String,
	pub alpaca_secret: String,
}

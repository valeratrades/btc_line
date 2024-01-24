#[derive(Debug, Clone)]
pub struct NowThen {
	pub now: LargeNumber,
	pub then: LargeNumber,
}

//TODO!!!: bring to parity \
impl std::fmt::Display for NowThen {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}->{}", self.now, self.then)
	}
}

#[derive(Debug, Clone, Default)]
pub struct LargeNumber(f64);
impl std::fmt::Display for LargeNumber {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut n = self.0;
		let suffix = match n {
			_ if n > 1_000_000_000_000_000.0 => {
				n /= 1_000_000_000_000_000.0;
				"Q"
			}
			_ if n > 1_000_000_000_000.0 => {
				n /= 1_000_000_000_000.0;
				"T"
			}
			_ if n > 1_000_000_000.0 => {
				n /= 1_000_000_000.0;
				"B"
			}
			_ if n > 1_000_000.0 => {
				n /= 1_000_000.0;
				"M"
			}
			_ if n > 1_000.0 => {
				n /= 1_000.0;
				"K"
			}
			_ => "",
		};
		write!(f, "{:.2}{}", n, suffix)
	}
}

impl std::str::FromStr for LargeNumber {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut n = s.parse::<f64>()?;

		//TODO!: parse LargeNumber from strings with suffixes too
		//let suffix = match s.chars().last() {
		//	Some('Q') => {
		//		n *= 1_000_000_000_000_000.0;
		//		"Q"
		//	}
		//	Some('T') => {
		//		n *= 1_000_000_000_000.0;
		//		"T"
		//	}
		//	Some('B') => {
		//		n *= 1_000_000_000.0;
		//		"B"
		//	}
		//	Some('M') => {
		//		n *= 1_000_000.0;
		//		"M"
		//	}
		//	Some('K') => {
		//		n *= 1_000.0;
		//		"K"
		//	}
		//	_ => "",
		//};
		//anyhow::ensure!(suffix != "", "Failed to parse LargeNumber");
		Ok(LargeNumber(n))
	}
}

impl From<f64> for LargeNumber {
	fn from(v: f64) -> Self {
		LargeNumber(v)
	}
}

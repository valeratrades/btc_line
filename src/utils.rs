#[derive(Debug, Clone)]
pub struct NowThen {
	pub now: f64,
	pub then: f64,
}

//TODO!!!: bring to parity \
impl std::fmt::Display for NowThen {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:.2}->{:.2}", self.now, self.then)
	}
}

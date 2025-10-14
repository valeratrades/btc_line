use tokio::{
	select,
	time::{Duration, interval},
};

#[tokio::main]
async fn main() {
	// trying to figure out correct select loop for main logic: some requests take very long, don't want them to be aborted

	let mut hello_tick = interval(Duration::from_secs(1));
	let mut world_tick = interval(Duration::from_secs(5));

	loop {
		select! {
			_ = hello_tick.tick() => println!("hello"),
			_ = world_tick.tick() => println!("world"),
		}
	}
}

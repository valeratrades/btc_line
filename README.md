# btc_line
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.92+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/btc_line.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/btc_line)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/btc_line)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/btc_line/errors.yml?branch=master&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/btc_line/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/btc_line/warnings.yml?branch=master&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/btc_line/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->

![Screenshot](./docs/.readme_assets/assets/screenshot.png)

```mermaid
graph TD
    base.cv::user["**User**<br>[External]"]
    base.cv::crypto_exchanges["**Cryptocurrency Exchanges**<br>/home/v/s/btc_line/Cargo.toml `v_exchanges = { version = #quot;=0.17.0#quot; }`, /home/v/s/btc_line/src/main.rs `use v_exchanges::{Exchange, binance::Binance};`, /home/v/s/btc_line/src/main_line.rs `use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};`"]
    subgraph base.cv::btc_line_app_boundary["**btc_line Application**<br>[External]"]
        base.cv::main_line_processor["**Main Line Processor**<br>/home/v/s/btc_line/src/main_line.rs `pub struct MainLine`, /home/v/s/btc_line/src/main_line.rs `fn create_ws_connection`, /home/v/s/btc_line/src/main_line.rs `fn handle_lsr`"]
        base.cv::additional_line_processor["**Additional Line Processor**<br>/home/v/s/btc_line/src/additional_line.rs `pub struct AdditionalLine`, /home/v/s/btc_line/src/additional_line.rs `fn get_open_interest_change`, /home/v/s/btc_line/src/additional_line.rs `fn get_btc_volume_change`"]
        base.cv::output_formatter["**Output Formatter**<br>/home/v/s/btc_line/src/output.rs `pub struct Output`, /home/v/s/btc_line/src/output.rs `fn output`"]
        %% Edges at this level (grouped by source)
        base.cv::main_line_processor["**Main Line Processor**<br>/home/v/s/btc_line/src/main_line.rs `pub struct MainLine`, /home/v/s/btc_line/src/main_line.rs `fn create_ws_connection`, /home/v/s/btc_line/src/main_line.rs `fn handle_lsr`"] -->|"Sends formatted data to"| base.cv::output_formatter["**Output Formatter**<br>/home/v/s/btc_line/src/output.rs `pub struct Output`, /home/v/s/btc_line/src/output.rs `fn output`"]
        base.cv::additional_line_processor["**Additional Line Processor**<br>/home/v/s/btc_line/src/additional_line.rs `pub struct AdditionalLine`, /home/v/s/btc_line/src/additional_line.rs `fn get_open_interest_change`, /home/v/s/btc_line/src/additional_line.rs `fn get_btc_volume_change`"] -->|"Sends formatted data to"| base.cv::output_formatter["**Output Formatter**<br>/home/v/s/btc_line/src/output.rs `pub struct Output`, /home/v/s/btc_line/src/output.rs `fn output`"]
    end
    %% Edges at this level (grouped by source)
    base.cv::main_line_processor["**Main Line Processor**<br>/home/v/s/btc_line/src/main_line.rs `pub struct MainLine`, /home/v/s/btc_line/src/main_line.rs `fn create_ws_connection`, /home/v/s/btc_line/src/main_line.rs `fn handle_lsr`"] -->|"Pulls Trade Data"| base.cv::crypto_exchanges["**Cryptocurrency Exchanges**<br>/home/v/s/btc_line/Cargo.toml `v_exchanges = { version = #quot;=0.17.0#quot; }`, /home/v/s/btc_line/src/main.rs `use v_exchanges::{Exchange, binance::Binance};`, /home/v/s/btc_line/src/main_line.rs `use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};`"]
    base.cv::main_line_processor["**Main Line Processor**<br>/home/v/s/btc_line/src/main_line.rs `pub struct MainLine`, /home/v/s/btc_line/src/main_line.rs `fn create_ws_connection`, /home/v/s/btc_line/src/main_line.rs `fn handle_lsr`"] -->|"Pulls LSR Data"| base.cv::crypto_exchanges["**Cryptocurrency Exchanges**<br>/home/v/s/btc_line/Cargo.toml `v_exchanges = { version = #quot;=0.17.0#quot; }`, /home/v/s/btc_line/src/main.rs `use v_exchanges::{Exchange, binance::Binance};`, /home/v/s/btc_line/src/main_line.rs `use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};`"]
    base.cv::additional_line_processor["**Additional Line Processor**<br>/home/v/s/btc_line/src/additional_line.rs `pub struct AdditionalLine`, /home/v/s/btc_line/src/additional_line.rs `fn get_open_interest_change`, /home/v/s/btc_line/src/additional_line.rs `fn get_btc_volume_change`"] -->|"Pulls OI and Volume Data"| base.cv::crypto_exchanges["**Cryptocurrency Exchanges**<br>/home/v/s/btc_line/Cargo.toml `v_exchanges = { version = #quot;=0.17.0#quot; }`, /home/v/s/btc_line/src/main.rs `use v_exchanges::{Exchange, binance::Binance};`, /home/v/s/btc_line/src/main_line.rs `use v_exchanges::{Binance, Exchange as _, ExchangeResult, ExchangeStream, Instrument, Trade, adapters::generics::ws::WsError};`"]
    base.cv::user["**User**<br>[External]"] -->|"Receives data from"| base.cv::output_formatter["**Output Formatter**<br>/home/v/s/btc_line/src/output.rs `pub struct Output`, /home/v/s/btc_line/src/output.rs `fn output`"]
```

## Usage
```sh
btc_line start
```



<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting). For project's architecture, see <a href="./docs/ARCHITECTURE.md">ARCHITECTURE.md</a>.
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>


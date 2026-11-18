# btc_line
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.92+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/btc_line.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/btc_line)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/btc_line)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-loc.json)
![Build time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-build.json)
![Test time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-test.json)
<br>
[![errors](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-errors.json)](https://github.com/valeratrades/btc_line/actions/workflows/errors.yml?query=branch%3Amain)
[![warnings](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/btc_line-warnings.json)](https://github.com/valeratrades/btc_line/actions/workflows/warnings.yml?query=branch%3Amain)

![Screenshot](./docs/.readme_assets/assets/screenshot.png)
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
</summary>

TODO


</details>
<!-- markdownlint-restore -->

## Usage
```sh
btc_line start
```

## Architecture

```mermaid
flowchart LR
    bn[("Binance")]

    subgraph app["btc_line"]
        direction LR
        main["<b>MainLine</b><br/>price · LSR"]
        add["<b>AdditionalLine</b><br/>OI Δ · volume Δ"]
        out["<b>Output</b><br/>dedup · rate-limit"]
        main --> out
        add --> out
    end

    pipes[/"$XDG_STATE_HOME/btc_line/{main,additional}"/]
    eww[["eww widgets"]]

    bn -- "trades (ws)<br/>LSR every 15s" --> main
    bn -- "OI · volume<br/>every 15s" --> add
    out -- "outputs.pipes" --> pipes
    out -- "outputs.eww" --> eww
```


<br>

#### Dev links
- <sup><a href="./docs/ARCHITECTURE.md">architecture</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/STYLE.md">style</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/tree/main/code/conventions">conventions</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/conventions/git">contributing</a></sup>

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


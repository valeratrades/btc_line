# Setup
## Windows
...
## Not Windows
Good luck.

For SPY window to work you will have to get your demo api keys on https://app.alpaca.markets/paper/dashboard/overview -> API Keys, and put them into keys.json
Same for twelvedata

Script shows f"{BTC_price}|{%longs}" always.
If keys are connected, automatically adds a line for SPY on market open.
When main window clicked, adds another line with f"{%longs_topAccounts}{OI}+{OIchange-in-userChosenTF_defalt1h}
When additional line clicked, creates a window with LSR outliers, CME positions, some Volatility metrics and other fun stuff in the future.

For SPY window to work you will have to get your demo api keys on https://app.alpaca.markets/paper/dashboard/overview -> API Keys, and put them into config.json.

Script shows f"{BTC_price}|{%longs}" always.
If keys are connected, automatically adds a line for SPY on market open.
When main window clicked, adds another line with f"{%longs_topAccounts}*{OI}+{OIchange-in-userChosenTF_defalt1h}
When additional line clicked, creates a window with LSR outliers.

Additional functionality will be added to the additional_line and this window later.



Currently there are some random keys for SPY connection that will produce errors if more than one person is connected:
("key": "PKQE2J4W14D1HUDBD4LO",
"secret": "5cD3hvRKaiSppJdYjY5MOjNUbXj08IoNFgEslBJt")

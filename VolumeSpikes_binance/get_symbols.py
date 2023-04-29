import requests
import binance
import json
client = binance.Client("jCPgfgCVOpgNtjlodbkkxkFw8MMdVUAiX01d7g3BggiG5XacCUT3Bu7Ob9fvH6HQ", "ONsDZbLuQpxi3iwzO3ZMxmH5NEjEppnzh7ns4lVQp5M8JmyLMMR5Gs8kXhBEXu0L")
allAssets = client.get_all_isolated_margin_symbols()

isolated_margin_pairs = []
for chunk in allAssets:
    if chunk["quote"] == "USDT":
        isolated_margin_pairs.append(chunk["symbol"])
#len(isolated_margin_pairs) == 286. All pairs in cross margin list are also present in isolated pairs list
toRemove = ["BUSDUSDT", "USDCUSDT", "TUSDUSDT"]
for symbol in toRemove:
    isolated_margin_pairs.remove(symbol)
with open("margin_pairs.json", "w") as f:
    json.dump(isolated_margin_pairs, f)


info = client.futures_exchange_info()
futures_pairs = []
for chunk in info["symbols"]:
    '''check if quote is USDT'''
    if chunk["symbol"][-1] == "T":
        futures_pairs.append(chunk["symbol"])
#len(futures_pairs) == 140
toRemove = ["BTCSTUSDT"]
for symbol in toRemove:
    futures_pairs.remove(symbol)
with open("futures_pairs.json", "w") as f:
    json.dump(futures_pairs, f)

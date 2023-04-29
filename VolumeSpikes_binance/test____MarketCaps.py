import json, pprint, time
pp = pprint.PrettyPrinter()
import requests
import numpy as np
from requests import Request, Session
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects

with open("margin_pairs.json") as f:
    symbols = json.load(f)


url = 'https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest'
headers = {
  'Accepts': 'application/json',
  'X-CMC_PRO_API_KEY': '0537352d-28d7-4911-8870-4740497ef800',
}
session = Session()
session.headers.update(headers)


vs_session = Session()
def av_volume(symbol):
    params = {"symbol": symbol, "interval": "1d", "limit": "30"}
    data = vs_session.get("https://api.binance.com/api/v3/klines", params=params).json()
    volumes = []
    for chunk in data:
        volumes.append(float(chunk[7]))
    return np.average(volumes)

exceptions = ["BTTC"]
market_caps = {}
for i, symbol in enumerate(symbols):
    continue_ = False
    symbol = symbol[:-4]
    if symbol in exceptions:
        print(f"skipped -{symbol}-")
        continue
    params = {
      'symbol': symbol,
      'convert':'USD'
    }
    print(symbol)
    data = session.get(url, params=params).json()
    while data['status']['error_code'] != 0:
        print(data['status']['error_message'])
        if "Invalid value" in data['status']['error_message']:
            exceptions.append(symbol)
            print(exceptions)
            continue_ = True
            break
        time.sleep(20)
        data = session.get(url, params=params).json()
    if continue_ is True:
        continue
    market_cap = round(data['data'][f'{symbol}']['quote']['USD']['market_cap'])
    av_volume_ = round(av_volume(symbol+"USDT"))
    market_caps[f"{symbol}"] = market_cap, av_volume_
    #if (i+1)%30 == 0:
        #time.sleep(60)
with open("MarketCaps.json", "w") as f:
    json.dump(market_caps, f)


#params = {"symbol": "BTCUSDT", "interval": "1d", "limit": "30"}

#print(r)

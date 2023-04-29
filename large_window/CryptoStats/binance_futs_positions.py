import requests

r = requests.get("https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
globalLongShortAccountRatioDaily = r[:4]

r = requests.get("https://fapi.binance.com/futures/data/topLongShortPositionRatio?symbol=BTCUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
topLongShortPositionRatioDaily = r[:4]


r = requests.get("https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol=BCHUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
BCH = r[:3]

r = requests.get("https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol=ETCUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
ETC = r[:3]

r = requests.get("https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol=XRPUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
XRP = r[:3]

r = requests.get("https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol=ADAUSDT&period=1d&limit=1").json()
r = r[0]['longShortRatio']
ADA = r[:3]

print(globalLongShortAccountRatioDaily, '  ', topLongShortPositionRatioDaily)
print(f"           > BCH {BCH}, ETC {ETC}, XRP {XRP}, ADA {ADA}")


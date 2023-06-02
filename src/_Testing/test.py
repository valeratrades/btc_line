import requests

surl = 'https://api.binance.com/api/v3/klines'
furl = 'https://fapi.binance.com/fapi/v1/klines'
params = {
    'symbol': 'BTCUSDT',
    'interval': '1d',
    'limit': 7
}

r = requests.get(surl, params=params).json()

print(r)
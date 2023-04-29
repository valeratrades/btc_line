import requests

a = 'VIX:CBOE'
api_key='8d61e96c72ee431c94ba0d604a434d6a'
url = 'https://api.twelvedata.com/time_series?symbol=VIX:CBOE&interval=5min&output=1&apikey=8d61e96c72ee431c94ba0d604a434d6a'

r = requests.get(url).json()
close = r["values"][0]["close"]
print(close[:2])

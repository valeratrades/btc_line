import requests

r = requests.get("https://bitcoin-sentiment.augmento.ai/gauge/_dash-layout").json()
r = r["props"]["children"][0]["props"]["children"][0]["props"]["figure"]["data"][0]["value"]
value = format(r, '.3f')

print(value)

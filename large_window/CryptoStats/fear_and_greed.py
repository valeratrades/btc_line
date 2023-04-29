import requests

response = requests.get("https://api.alternative.me/fng/?limit=1")
js = response.json()
value = js['data'][0]['value']

print(value)

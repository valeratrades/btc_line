import time
from twelvedata import TDClient

def on_event(e):
    print(e)
def on_message(m):
    print(m)


td = TDClient(apikey="8d61e96c72ee431c94ba0d604a434d6a")
ws = td.websocket(on_event=on_event, on_message=on_message)
ws.subscribe(["BTC/USD", "ETH/USD"])
ws.connect()
while True:
    ws.heartbeat()
    time.sleep(10)
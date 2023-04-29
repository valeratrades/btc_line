import json, websocket, threading, time

def on_message(ws, message):
    print(message)

def on_open(ws):
    print("Connection established")

def connect_to_twelvedata():
    while True:
        try:
            ws = websocket.WebSocketApp("wss://fstream.binance.com/ws/btcusdt@markPrice",
                                        on_message=on_message,
                                        on_open=on_open)
            ws.run_forever()
        except Exception as e:
            print(f"WebSocket error: {e}")
            time.sleep(1)

t = threading.Thread(target=connect_to_twelvedata)
t.daemon = True
t.start()
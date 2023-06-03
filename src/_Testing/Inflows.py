import requests, pandas as pd, numpy as np
import tkinter as tk
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure

surl = 'https://api.binance.com/api/v3/klines'
furl = 'https://fapi.binance.com/fapi/v1/klines'
params = {
    'symbol': 'BTCUSDT',
    'interval': '1d',
    'limit': 7
}

r = requests.get(furl, params=params).json()

df = pd.DataFrame()
netInflows = []
for day in r:
    buys = float(day[-2])
    total = float(day[7])
    norm_net = 2*buys/total - 1

    netInflows.append(norm_net)
df['net_inflow'] = netInflows
print(netInflows)
df['timestamp'] = [x[0] for x in r]
df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
df.set_index('timestamp', inplace=True)



root = tk.Tk()
#root.overrideredirect(True)
fig = Figure(figsize=(21, 4), dpi=4, facecolor='black')
ax = fig.add_subplot(111, facecolor='black')
colors = ['green' if _y >=0 else 'red' for _y in df['net_inflow']]
ax.bar(df.index, df['net_inflow'], color=colors)

ax.set_xticklabels([])
ax.set_yticklabels([])
fig.subplots_adjust(left=0, bottom=0, right=1, top=1)

canvas = FigureCanvasTkAgg(fig, master=root)
canvas.draw()
canvas.get_tk_widget().pack()

root.mainloop()

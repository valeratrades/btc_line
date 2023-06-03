import requests, pandas as pd, tempfile, tkinter as tk
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure

tempdir = tempfile.gettempdir()

def create_fig(type):
    if type=='spot':
        url = 'https://api.binance.com/api/v3/klines'
        path = tempdir+'/SpotInflowFig.png'
    else:
        url = 'https://fapi.binance.com/fapi/v1/klines'
        path = tempdir+'/FutsInflowFig.png'
        
    params = {
        'symbol': 'BTCUSDT',
        'interval': '1d',
        'limit': 7
    }

    r = requests.get(url, params=params).json()

    df = pd.DataFrame()
    netInflows = []
    total_volume = 0
    for day in r:
        buys = float(day[-2])
        total = float(day[7])
        net = 2*buys-total
        norm_net = net/total
        total_volume+=total

        netInflows.append(net)
    df['net_inflow'] = netInflows
    weekInflow = sum(netInflows)
    print(weekInflow, weekInflow/total_volume)
    df['timestamp'] = [x[0] for x in r]
    df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms')
    df.set_index('timestamp', inplace=True)

    fig = Figure(figsize=(21, 4), dpi=3.8, facecolor='black')
    ax = fig.add_subplot(111, facecolor='black')
    colors = ['green' if _y >=0 else 'red' for _y in df['net_inflow']]
    ax.bar(df.index, df['net_inflow'], color=colors)

    ax.set_xticklabels([])
    ax.set_yticklabels([])
    fig.subplots_adjust(left=0, bottom=0, right=1, top=1)
    fig.savefig(path)
    return path
path = create_fig('spot')

# root = tk.Tk()
# root.overrideredirect(True)

# # from PIL import Image, ImageTk
# # img = Image.open(path)
# # photo = ImageTk.PhotoImage(img)
# # label = tk.Label(root, image=photo)
# # label.image = photo
# # label.pack()

# # canvas = FigureCanvasTkAgg(fig, master=root)
# # canvas.draw()
# # canvas.get_tk_widget().pack()

# root.mainloop()

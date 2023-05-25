import pandas as pd
import requests, json, threading, os, tempfile
from datetime import datetime, timedelta
import plotly.graph_objects as go
from IPython.display import Image

hours_selected = 24
timeframe = 5
script_dir = os.path.dirname(os.path.abspath(__file__))
tempdir = tempfile.gettempdir()


def get_historical_data(symbol):
    url = f"https://api.binance.com/api/v3/klines?symbol={symbol}&interval={timeframe}m"

    time_ago = datetime.now() - timedelta(hours=hours_selected)
    time_ago_ms = int(time_ago.timestamp() * 1000)
    url += f"&startTime={time_ago_ms}"

    raw_data = requests.get(url).json()
    df = pd.DataFrame(raw_data, columns=['open_time', 'open', 'high', 'low', 'close', 'volume', 'close_time',
                                         'quote_asset_volume', 'trades', 'taker_buy_base', 'taker_buy_quote', 'ignore'])
    df['open_time'] = pd.to_datetime(df['open_time'], unit='ms')
    df['close'] = pd.to_numeric(df['close'])
    df.set_index('open_time', inplace=True, drop=False)
    df['return'] = df['close'].pct_change() + 1
    df.iloc[0, df.columns.get_loc('return')] = 1 # set first datapoint to one
    df['cumulative_return'] = df['return'].cumprod()
    return df


def plot_market_structure(symbols):
    def fetch_data(symbol, data):
        try:
            df = get_historical_data(symbol)
            data[symbol] = df
        except Exception as e:
            print(f"Failed to fetch data for symbol: {symbol}. Error: {str(e)}")
    data = {}
    threads = []
    for symbol in symbols:
        thread = threading.Thread(target=fetch_data, args=(symbol, data))
        thread.start()
        threads.append(thread)
    for thread in threads:
        thread.join()

    normalized_data = {symbol: df['close'] / df['close'].iloc[0] for symbol, df in data.items()}
    normalized_df = pd.DataFrame(normalized_data)

    performance = normalized_df.iloc[-1] - normalized_df.iloc[0]
    top_performers = performance.nlargest(5).index
    bottom_performers = performance.nsmallest(5).index

    fig = go.Figure()

    for column in normalized_df.columns:
        if column not in top_performers and column not in bottom_performers and column != 'BTCUSDT':
            fig.add_trace(
                go.Scatter(
                    x=normalized_df.index,
                    y=normalized_df[column],
                    mode='lines',
                    name='',
                    line=dict(width=1, color='grey'),
                    showlegend=False
                )
            )
    for column in top_performers:
        fig.add_trace(
            go.Scatter(
                x=normalized_df.index,
                y=normalized_df[column],
                mode='lines',
                name=column,
                line=dict(width=2),
                showlegend=True
            )
        )
    fig.add_trace(
        go.Scatter(
            x=normalized_df.index,
            y=normalized_df['BTCUSDT'],
            mode='lines',
            name='~~BTC~~',
            line=dict(width=5, color='gold'),
            showlegend=True
        )
    )
    for column in bottom_performers[::-1]:
        fig.add_trace(
            go.Scatter(
                x=normalized_df.index,
                y=normalized_df[column],
                mode='lines',
                name=column,
                line=dict(width=2),
                showlegend=True
            )
        )
    fig.update_layout(template='plotly_dark', autosize=True, margin=dict(l=0, r=0, b=0, t=0))
    fig.update_xaxes(range=[normalized_df.index.min(), normalized_df.index.max()])
    fig.update_yaxes(range=[normalized_df.min().min(), normalized_df.max().max()])

    return fig

def main():
    symbols = json.load(open(os.path.join(tempdir, 'allListed.json')))

    fig = plot_market_structure(symbols)
    fig.write_image(os.path.join(tempdir, 'MarketStructure.png'))

if __name__ == "__main__":
    main()

import os, json, time, logging, threading, tempfile
from alpaca_trade_api.stream import Stream

log = logging.getLogger(__name__)

config = json.load(open('config.json', 'r'))
API_KEY = config['alpaca']['key']
API_SECRET = config['alpaca']['secret']
os.environ['APCA_API_KEY_ID'] = API_KEY
os.environ['APCA_API_SECRET_KEY'] = API_SECRET
tempdir = tempfile.gettempdir()
shared_data_path = os.path.join(tempdir, 'spy_feed.json')

async def dump_data(t):
    json.dump((time.time(), t['p']), open(shared_data_path, 'w'))

def drop_timestamp(interval):
    while True:
        last_timestamp = json.loads(os.environ.get('SPY_FEED_DATA', json.dumps((0, None))))[0]
        if last_timestamp + interval < time.time():
            os.environ['SPY_FEED_DATA'] = json.dumps((time.time(), None))
        time.sleep(interval)

def main():
    logging.basicConfig(format='%(asctime)s %(message)s', level=logging.INFO)
    feed = 'iex'
    stream = Stream(data_feed=feed, raw_data=True)
    stream.subscribe_trades(dump_data, 'SPY')

    @stream.on_status("*")
    async def _(status):
        print('status', status)

    timestamp_thread = threading.Thread(target=drop_timestamp, args=(30,))
    timestamp_thread.start()

    stream.run()

if __name__ == "__main__":
    main()

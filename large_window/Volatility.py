import asyncio, json, tempfile, os, time, datetime
from aiohttp import ClientSession

tempdir = tempfile.gettempdir()
settings = json.load(open(os.path.join(tempdir, 'settings.json')))
keys = json.load(open(os.path.join(tempdir, 'keys.json')))

async def get_VIX(session):
    key = keys['twelvedata']['key']

    async def get_last_close():
        url = f'https://api.twelvedata.com/quote?symbol=VIX:CBOE&apikey={key}'
        async with session.get(url) as response:
            r = await response.json()

        global VIX_closed_hours_ago, VIX_close
        VIX_closed_hours_ago = (time.time() - int(r["timestamp"])) / 3600
        VIX_close = float(r["close"])

        return VIX_close

    async def get_then():
        global VIX_closed_hours_ago, VIX_close
        limit = settings['comparison_limit']
        if VIX_closed_hours_ago > limit:
            return VIX_close

        url = f'https://api.twelvedata.com/time_series?symbol=VIX:CBOE&interval=1h&outputsize={limit}&apikey={key}'
        async with session.get(url) as response:
            r = await response.json()

        points = r['values']
        then = datetime.datetime.now() - datetime.timedelta(hours=limit)
        distance = abs(then - datetime.datetime.strptime(points[0]['datetime'], "%Y-%m-%d %H:%M:%S"))
        closest_i = 0
        for i, p in enumerate(points):
            datetime_obj = datetime.datetime.strptime(p['datetime'], "%Y-%m-%d %H:%M:%S")
            if abs(then - datetime_obj) < distance:
                distance = abs(then - datetime_obj)
                closest_i = i

        return float(points[closest_i]['close'])

    last_close = await get_last_close()
    change = last_close - await get_then()
    format = f"{last_close}{change:+}" if change else f"{last_close}"

    return format

async def main():
    async with ClientSession() as session:
        out = await get_VIX(session)  # TODO: add BVOL and greeks
        config = json.load(open(os.path.join(tempdir, 'large_window.json'), 'r'))
        config['Volatility'] = out
        json.dump(config, open(os.path.join(tempdir, 'large_window.json'), 'w'))

asyncio.run(main())
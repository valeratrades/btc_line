import requests, aiohttp, asyncio, tempfile, json, os


most_longed = 10
most_shorted = 10
#========================================================== 

tempdir = tempfile.gettempdir()

exchangeInfo = requests.get('https://fapi.binance.com/fapi/v1/exchangeInfo').json()
futures_pairs = []
for chunk in exchangeInfo['symbols']:
    if chunk["symbol"][-1] == "T":
        futures_pairs.append(chunk["symbol"])
toRemove = ["BTCSTUSDT"]
for symbol in toRemove:
    futures_pairs.remove(symbol)

async def get_ratio(session, symbol):
    try:
        async with session.get(f"https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol={symbol}&period=5m&limit=1") as resp:
            r = await resp.json()
            r = r[0]['longShortRatio']
            if not r==float('inf'):
                ratios.append((symbol, r[:3]))
    except:
        pass

ratios = []
async def main(symbols):
    async with aiohttp.ClientSession() as session:
        tasks = []
        for symbol in symbols:
            task = asyncio.create_task(get_ratio(session, symbol))
            tasks.append(task)

        await asyncio.gather(*tasks)
        sorted_ratios = sorted(ratios, key=lambda x: float(x[1]))
        
        result_string = 'Most Longed:          Most Shorted:'
        for r in range(most_longed):
            m_pair = sorted_ratios[-r-1][0][:-4]+':'
            l_pair = sorted_ratios[r][0][:-4]+':'
            second_row = f'       ├{l_pair:<9} {sorted_ratios[r][1]}' if r < most_shorted else ''
            result_string += f'\n     ├{m_pair:<9} {sorted_ratios[-r-1][1]}{second_row}'
        
        large_window = json.load(open(os.path.join(tempdir,'large_window.json'), 'r'))
        large_window['LSRoutliers'] = result_string
        json.dump(large_window, open(os.path.join(tempdir,'large_window.json'), 'w'))

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
asyncio.run(main(futures_pairs))

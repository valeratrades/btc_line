import requests, aiohttp, asyncio

#<Settings>
most_leveraged = 10
least_leveraged = 10
#<\Settings>

exchangeInfo = requests.get('https://fapi.binance.com/fapi/v1/exchangeInfo').json()
futures_pairs = []
for chunk in exchangeInfo['symbols']:
    if chunk["symbol"][-1] == "T":
        futures_pairs.append(chunk["symbol"])
toRemove = ["BTCSTUSDT", "FTTUSDT"]
for symbol in toRemove:
    futures_pairs.remove(symbol)

async def get_ratio(session, symbol):
    async with session.get(f"https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol={symbol}&period=1d&limit=1") as resp:
        r = await resp.json()
        r = r[0]['longShortRatio']
        ratios.append((symbol, r[:3]))

ratios = []
async def main(symbols):
    async with aiohttp.ClientSession() as session:
        tasks = []
        for symbol in symbols:
            task = asyncio.create_task(get_ratio(session, symbol))
            tasks.append(task)

        await asyncio.gather(*tasks)
        sorted_ratios = sorted(ratios, key=lambda x: float(x[1]))
        print('\nMost Leveraged:      Least Leveraged:')
        for r in range(most_leveraged):
            m_pair = sorted_ratios[-r-1][0][:-4]+':'
            l_pair = sorted_ratios[r][0][:-4]+':'
            second_row = f'       ├{l_pair:<9} {sorted_ratios[r][1]}' if r < least_leveraged else ''
            print(f'     ├{m_pair:<9} {sorted_ratios[-r-1][1]}{second_row}')

asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
asyncio.run(main(futures_pairs))

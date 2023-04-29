import requests, time, numpy as np, math, json, asyncio, aiohttp #, os
from time import sleep
from playsound import playsound

#NB <doesn't work>
import warnings
def fxn():
    warnings.warn("coroutine 'get_vol_sma' was never awaited", RuntimeWarning)

with warnings.catch_warnings():
    warnings.simplefilter("ignore")
    #fxn()
#NB <\doesn't work>

import win32gui
from pynput.keyboard import Key, Controller
keyboard = Controller()

from asyncio import TimeoutError
from aiohttp import ServerDisconnectedError, ClientOSError
from urllib3.exceptions import ReadTimeoutError
from requests.exceptions import ReadTimeout, ConnectionError as RCE

from spikesMessagesDeletor import clear_alerts
clear_alerts(18)

#<inputs>
from get_symbols import isolated_margin_pairs, futures_pairs
                                    
computedLength = 30 #                                           default computed length is 30
length = 50
allowedPenaltyPoints = -1 #3

timesAverageVol = 4 #                                          default is 4. Variable determines by how many times vol should be higher than av to count
pause = 0.3 #                                                  time in seconds, which script sleeps on each symbol. Keep in mind - you can only send 1200 requests per minute. And binance server closes the connection after 1 minute of it running, so keep the pause under 60/146
pause_between_market_types = 0 #                                time.sleep beetwen margin and futures circles
Except = [] #                                                   pairs on which I wanna receive alerts more than once. Format: "BTCUSDT" or "BTCUSDTPERP

#</inputs>


#<send tg msg>
import telebot
API_KEY = '5131848746:AAEk1LuXl7_0fdN5WzA956t_jjo8Pn6cbl8'
tb = telebot.TeleBot(API_KEY, False)
def send_msg(symbol):
    tb.send_message(-1001179171854, f"Volume spikes on {symbol}")
#</send tg msg>
    
def assignCharacter(num):
    seventh = math.floor(num*7)/7
    if seventh < 0.14:
        return "▁"
    if 0.14 < seventh < 0.15:
        return "▂"
    if 0.28 < seventh < 0.29:
        return "▃"
    if 0.42 < seventh < 0.43:
        return "▃"
        '''4/8 block doesnt work properly'''
    if 0.57 < seventh < 0.58:
        return "▅"
    if 0.71 < seventh < 0.72:
        return "▆"
    if 0.85 < seventh < 0.86:
        return "▇"
    if seventh >= 1:
        return "█"

def open_chart(symbol):
    current_hwnd = win32gui.GetForegroundWindow()
    def winEnumHandler(hwnd, ctx):
        if win32gui.IsWindowVisible(hwnd):
            #   Highly breakable
            if '% Без названия - Google Chrome' in win32gui.GetWindowText(hwnd) \
            and not symbol[1].isnumeric():
                playsound("Notification.mp3")
                sleep(0.05)
                win32gui.SetForegroundWindow(hwnd)

                for char in symbol:
                    keyboard.press(char)
                    keyboard.release(char)
                    
                sleep(0.01)     
                keyboard.press(Key.enter)
                keyboard.release(Key.enter)

                keyboard.press(Key.alt)
                keyboard.press('w')
                keyboard.release('w')
                keyboard.release(Key.alt)
                sleep(0.01)
                
                try:
                    win32gui.SetForegroundWindow(current_hwnd)
                except:
                    pass
    win32gui.EnumWindows(winEnumHandler, None)


last_symbol = ""      # for error message
async def get_vol_sma(market, symbol, session):
    global last_symbol
    last_symbol = symbol
    url = margin_url if market == 'margin' else futures_url
    params = {"symbol": symbol, "interval": "1d", "limit": "30"}
    async with session.get(url, params=params) as resp:
        data = await resp.json()
        if resp.status == 429:
            time.sleep(60)
            print("got warning 429")
        if resp.status == 418:
            print("you've been banned")
            time.sleep(180)
        
        volumes = []
        for chunk in data:
            volumes.append(float(chunk[7])) if chunk[7]!='0.0' else volumes.append(0.0)
        av_volume = np.mean(volumes)
        return av_volume/vol_smaDivider

async def plot(market, symbol, session):
    task1 = get_vol_sma(market, symbol, session)
    url = futures_url if market == 'futures' else margin_url
    
    params = {"symbol": symbol, "interval": "1m", "limit": str(length)}
    async with session.get(url, params=params) as resp:
        data = await resp.json()
        penaltyPoints = 0
        aLittleChart = ""
        vol_sma = await task1
        for i, chunk in enumerate(data):
            quoteVolume = float(chunk[7])

            if quoteVolume == 0.0:
                aLittleChart += "▁"
                #                                                       doesnt assign a penaltyPoint in this case. But i'm leaving it like this to be able to spot stalled pairs and remove them
            else:
                aLittleChart += assignCharacter(quoteVolume/vol_sma)
            if quoteVolume < vol_sma:
                if i >= offset:
                    penaltyPoints += 1
    time.sleep(pause)
    print(symbol[:-4].ljust(6), aLittleChart)

    if market == 'margin':
        if penaltyPoints <= allowedPenaltyPoints and not symbol in spiked_coins:
                print(f"Spikes on margin      --------------{symbol.ljust(10,'-')}-----------")
                open_chart(symbol)
                send_msg(symbol[:-4])
                spiked_coins.append(symbol)
    else:
        if penaltyPoints <= allowedPenaltyPoints and not symbol+"PERP" in spiked_coins:
                print(f"Spikes on futures     --------------{symbol.ljust(10,'-')}-----------")
                open_chart(symbol+"PERP")
                send_msg(symbol[:-4]+" PERP")
                spiked_coins.append(symbol+"PERP")

margin_url = "https://api.binance.com/api/v3/klines"
futures_url = "https://fapi.binance.com/fapi/v1/klines"
offset = length - computedLength
vol_smaDivider = 1440 / timesAverageVol
spiked_coins = []
async def main():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        while True:
            try:
                print("Margin:\n")
                array_split = np.array_split(isolated_margin_pairs, 2)
                first_half, second_half = array_split[0], array_split[1]
                
                async with aiohttp.ClientSession() as session:
                    tasks = []
                    for symbol in first_half:
                        task = asyncio.create_task(plot('margin', symbol, session))
                        tasks.append(task)      
                    await asyncio.gather(*tasks)
                #   binance server interupts my session after one minute of its activity. So if you wanna have longer pauses, split the execution into more connection sessions
                async with aiohttp.ClientSession() as session:
                    tasks = []
                    for symbol in second_half:
                        task = asyncio.create_task(plot('margin', symbol, session))
                        tasks.append(task)      
                    await asyncio.gather(*tasks)
                    
                time.sleep(pause_between_market_types)
                
                print("Futures:\n")
                async with aiohttp.ClientSession() as session:
                    tasks = []
                    for symbol in futures_pairs:
                        task = asyncio.create_task(plot("futures", symbol, session))
                        tasks.append(task)
                    await asyncio.gather(*tasks)
                        
                for symbol in Except:
                    spiked_coins.remove(symbol)
                with open("Spiked_Coins.json", "w") as f:
                    json.dump(spiked_coins, f)
                time.sleep(pause_between_market_types)
            except (RCE, ReadTimeout, ReadTimeoutError, OSError, RuntimeWarning, ConnectionResetError, ConnectionAbortedError, ServerDisconnectedError, TimeoutError, ConnectionError, ClientOSError, ConnectionResetError) as e:
                print(e)
                print(f"error appeared on {last_symbol}")
                #send_msg(f"{e} error appeared on {symbol}")
                time.sleep(15)
                pass

asyncio.run(main()) 
    


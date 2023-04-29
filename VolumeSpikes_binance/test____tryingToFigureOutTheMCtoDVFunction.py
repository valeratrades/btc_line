import json
from matplotlib import pyplot as plt

with open("MarketCaps.json", "r") as f:
    caps_dict = json.load(f)

caps_dict = dict(sorted(caps_dict.items(), key=lambda item: item[1], reverse=True))
caps_dict.popitem()

ratios, market_caps = [], []
for symbol in caps_dict:
    ratio = caps_dict[symbol][0]/caps_dict[symbol][1]
    ratios.append(ratio)
    market_cap = caps_dict[symbol][0]
    market_caps.append(market_cap)
    print(f"{str(market_cap).ljust(17)}    {ratio:.0f}")

axis = [i for i in range(len(ratios))]
print(axis)
plt.plot(axis, ratios)
plt.show()

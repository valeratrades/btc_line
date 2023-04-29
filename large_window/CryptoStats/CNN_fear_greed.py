import requests
from bs4 import BeautifulSoup

r = requests.get("https://money.cnn.com/data/fear-and-greed/", headers={"User-Agent": "Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.10; rv:86.1) Gecko/20100101 Firefox/86.1"}).text
soup = BeautifulSoup(r, "lxml")

index_data = (
            soup.findAll("div", {"class": "modContent feargreed"})[0]
            .contents[0]
            .text
        )

def index_value():
    for i, l in enumerate(index_data):
        if index_data[i].isnumeric():
            value = index_data[i]
            if index_data[i+1].isnumeric():
                value += index_data[i+1]
            if index_data[i+2].isnumeric():
                value += index_data[i+2]
            return value

print(index_value())

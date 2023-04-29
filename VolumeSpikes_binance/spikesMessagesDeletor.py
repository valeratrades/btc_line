chat = -1001179171854
api_id = 19721916
api_hash = "18648e0f5f6a667047e2d3b92f756d69"

from telethon.sync import TelegramClient

client = TelegramClient('session_id', api_id, api_hash)
def clear_alerts(length):
    with client:
        # length is the limit on how many messages to fetch. Remove or change for more.
        for msg in client.iter_messages(chat, length):
            if "Volume spikes on" in msg.text:
                client.delete_messages(-1001179171854, msg.id)
    print(f"cleaned alerts {length} messages deep")

if __name__=='__main__':
    length = int(input('How deep you wish to clean? '))
    clear_alerts(length)

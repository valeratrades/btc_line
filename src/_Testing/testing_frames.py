import tkinter as tk

def create_button(window, text):
    button = tk.Button(window, text=text)
    button.pack(side='left')

window = tk.Tk()
frame = tk.Frame(window)
frame.pack()

create_button(frame, "Button 1")
create_button(frame, "Button 2")

window.mainloop()

from tkinter import Tk, Label
from PIL import Image, ImageTk
import tempfile, os
tempdir = tempfile.gettempdir()

# Create the Tkinter window
root = Tk()

# Open the image file
img = Image.open(os.path.join(tempdir, 'MarketStructure.png'))

# Convert the image to a Tkinter-compatible PhotoImage
tk_img = ImageTk.PhotoImage(img)

# Create a label to display the image
label = Label(root, image=tk_img)

# Pack the label into the window
label.pack()

# Run the window's main loop
root.mainloop()

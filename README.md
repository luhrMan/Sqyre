This tool only works on 2560 x 1440 because of hard-set values. (Hopefully this can change with an implementation of SVG?)
# What is it
Squire is a Macro Builder built for Dark and Darker. Squire is written using GO, with 2 notable libraries:
- Fyne (GUI)
- Robotgo (Automation)
    - Bitmap (Image Search)
- Gosseract (OCR)
    
The structure of the fyne `widget.Tree`:
- (Root) 1 Loop Action
- (Branch) Action with SubAction (Advanced Actions)
    - Loop actions
    - Image Search Action
    - OCR Action
- (Leaf) Action
  - `Click`: click the mouse where cursor is at
  - `Move`: move the mouse to specific coordinates
  - `Key`: Set a key state Up/Down
  - `Wait`: Wait for time set in milliseconds
# Why
fuck all that clicking

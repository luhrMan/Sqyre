# What is it
Squire is a Macro Builder built for Dark and Darker. Squire is written using GO, with 2 notable libraries:
    - Fyne (GUI)
    - Robotgo (Automation)
The structure of the fyne `widget.Tree`:
    - (Root) Macro
    - (Branch) Sequence
        - Can loop multiple times
    - (Leaf) Action
        - `Click`: click the mouse where cursor is at
        - `Move`: move the mouse to specific coordinates
        - `Key`: Set a key state Up/Down
        - `Sleep`: Sleep for time set in milliseconds
        - `Image Search`: Not impletmented, unsure how to approach this
        - `OCR`: Not implemented
# Why
fuck all that clicking
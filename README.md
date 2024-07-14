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

# BUILD INSTRUCTIONS
## Windows 10
Install Msys2
- Using the mingw64 console, install these packages
    - [mingwx86 toolchain](https://packages.msys2.org/groups/mingw-w64-x86_64-toolchain)
    - [gcc](https://packages.msys2.org/package/mingw-w64-x86_64-gcc)
    - optional if u want go in the same place[go](https://packages.msys2.org/package/mingw-w64-x86_64-go?repo=mingw64)
    - [opencv](https://packages.msys2.org/package/mingw-w64-x86_64-opencv)
    - [zlib](https://packages.msys2.org/package/mingw-w64-x86_64-zlib)
    - [tesseract](https://packages.msys2.org/package/mingw-w64-x86_64-tesseract-ocr)
    - [leptonica](https://packages.msys2.org/package/mingw-w64-x86_64-leptonica)
- set TESSDATA_PREFIX in your PATH, run this in mingw64 console `export TESSDATA_PREFIX=C:\msys64\mingw64\share\tessdata`

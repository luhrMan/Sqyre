# What is it

Sqyre is a Macro Builder originally built for Dark and Darker. Sqyre is written using GO, with a few notable libraries:

- Fyne (GUI)
- Robotgo (Automation)
- Gosseract aka Tesseract (OCR)
- GoCV aka OpenCV (Computer Vision)

The structure of the fyne `widget.Tree`:

- (Root) 1 Loop Action
- (Branch) Action with SubAction (Advanced Actions)
    - `Loop`
    - `Image Search`
    - `OCR`
- (Leaf) Action
    - `Click`: click the mouse where cursor is at
    - `Move`: move the mouse to specific coordinates
    - `Key`: Set a key state Up/Down
    - `Wait`: Wait for time set in milliseconds

# Main Screen
<img width="2562" height="1362" alt="Screenshot from 2026-01-13 13-09-30" src="https://github.com/user-attachments/assets/53acf1a0-bc89-43d9-a7ab-856b46c3be63" />

# ImageSearch in action
![sqyre-imagesearch](https://github.com/user-attachments/assets/1a0fc8f4-06bb-4667-bb49-b1c4b2d5b508)

# Why

fuck all that clicking

# BUILD INSTRUCTIONS

## Linux
1. install dependencies
    - `sudo apt install tesseract-ocr libgl1-mesa-dev libx11-dev libx11-xcb-dev libxtst-dev libxcursor-dev libxrandr-dev libxinerama-dev g++ clang libtesseract-dev libxxf86vm-dev libxkbcommon-x11-dev golang-go cmake`
1. install opencv
    1. install gocv from this project folder
        - `go get -u -d gocv.io/x/gocv`
    1. install opencv
        - `cd $GOPATH/pkg/mod/gocv.io/x/gocv@v0.42.0`
        - `make install`

## Windows 10

Install Msys2

- Using the mingw64 console, install these packages
    - [mingwx86 toolchain](https://packages.msys2.org/groups/mingw-w64-x86_64-toolchain)
    - [gcc](https://packages.msys2.org/package/mingw-w64-x86_64-gcc)
    - optional if u want go in the same place[go](https://packages.msys2.org/package/mingw-w64-x86_64-go?repo=mingw64)
    - [opencv](https://packages.msys2.org/package/mingw-w64-x86_64-opencv)
    - [zlib](https://packages.msys2.org/package/mingw-w64-x86_64-zlib)
    - [tesseract](https://packages.msys2.org/package/mingw-w64-x86_64-tesseract-ocr)
    - [leptonica](http1s://packages.msys2.org/package/mingw-w64-x86_64-leptonica)
- download [english tessdata](https://github.com/tesseract-ocr/tessdata/blob/main/eng.traineddata)
- move `traineddata` to `C:\msys64\mingw64\share\tessdata`
- run these commands in the mingw64 console:
    - set TESSDATA_PREFIX
        - `export TESSDATA_PREFIX=C:\msys64\mingw64\share\tessdata`
    - if downloaded `go` in msys2, run these commands 
    - set GOROOT & GOPATH
        - `export GOROOT=/mingw64/lib/go`
        - `export GOPATH=/mingw64`
Add Msys2 console to VSCode
    - edit terminal settings
        - search for terminal integrated profiles and edit the windows settings
    - https://stackoverflow.com/questions/45836650/how-do-i-integrate-msys2-shell-into-visual-studio-code-on-window

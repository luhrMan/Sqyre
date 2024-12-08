package structs

import (
        "Squire/internal/utils"
        "fmt"
        "github.com/go-vgo/robotgo"
        "gocv.io/x/gocv"
        "image"
        "image/color"
)

func stashInventorySlots() {
        img := gocv.IMRead("./images/empty-stash.jpeg", gocv.IMReadColor)
        if img.Empty() {
                fmt.Println("Error reading main image")
        }
        defer img.Close()
        invRows := 20
        invCols := 12
        xSize := img.Cols() / invCols
        ySize := img.Rows() / invRows
        var invSlots []image.Rectangle
        //box := image.Rectangle{image.Point{0,0}, image.Point{img.Rows() / 12, img.Cols() / 24}}
        for r := 0; r < invCols; r++ {
                for c := 0; c < invRows; c++ {
                        invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
                }
        }
        for _, rect := range invSlots {
                windowSlot := gocv.NewWindow("slot")
                defer windowSlot.Close()
                windowSlot.IMShow(img.Region(rect))
                gocv.WaitKey(0)
                gocv.Rectangle(&img, rect, color.RGBA{R: 255, A: 255}, 2)
        }
        window := gocv.NewWindow("inventory ")
        defer window.Close()
        window.IMShow(img)
        gocv.WaitKey(0)
}

func merchantPlayerInventorySlots() {
        sb := GetSearchBox("Player Inventory Merchant")
        w := sb.RightX - sb.LeftX
        h := sb.BottomY - sb.TopY
        capture := robotgo.CaptureScreen(sb.LeftX+utils.XOffset, sb.TopY+utils.YOffset, w, h)
        robotgo.SaveJpeg(robotgo.ToImage(capture), "./images/search-area.jpeg")
        defer robotgo.FreeBitmap(capture)

        img := gocv.IMRead("./images/search-area.jpeg", gocv.IMReadColor)
        if img.Empty() {
                fmt.Println("Error reading main image")
        }
        defer img.Close()

        invRows := 5
        invCols := 10
        xSize := img.Cols() / invCols
        ySize := img.Rows() / invRows

        //	var invSlotMats []gocv.Mat
        //	for i, s := range invSlots {
        //		invSlotMats = append(invSlotMats, img.Region(s))
        //		defer invSlotMats[i].Close()
        //	}

        var invSlots []image.Rectangle
        for r := 0; r < invCols; r++ {
                for c := 0; c < invRows; c++ {
                        invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
                }
        }
}

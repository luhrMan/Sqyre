package utils

import (
        "Squire/internal/structs"
        "errors"
        "fmt"
        "github.com/go-vgo/robotgo"
        "gocv.io/x/gocv"
        "image"
        "image/color"
        "log"
)

func CalibrateInventorySearchboxes() {
        path := "./internal/resources/images/corners/bigger/"
        var (
                //                stashTLC  = gocv.IMRead(path+"stashCorner-TopLeft.png", gocv.IMReadColor)
                //                stashTRC  = gocv.IMRead(path+"stashCorner-TopRight.png", gocv.IMReadColor)
                //                stashBLC  = gocv.IMRead(path+"stashCorner-BottomLeft.png", gocv.IMReadColor)
                //                stashBRC  = gocv.IMRead(path+"stashCorner-BottomRight.png", gocv.IMReadColor)
                playerTLC = gocv.IMRead(path+"playerCorner-TopLeft.png", gocv.IMReadColor)
                playerTRC = gocv.IMRead(path+"playerCorner-TopRight.png", gocv.IMReadColor)
                playerBLC = gocv.IMRead(path+"playerCorner-BottomLeft.png", gocv.IMReadColor)
                playerBRC = gocv.IMRead(path+"playerCorner-BottomRight.png", gocv.IMReadColor)
        )
        //        stashInvLocation(stashTLC, stashTRC, stashBLC, stashBRC)
        stashPlayerInvLocation(playerTLC, playerTRC, playerBLC, playerBRC)
        //        merchantInvLocation(stashTLC, stashTRC, stashBLC, stashBRC)
        //        merchantPlayerInvLocation(playerTLC, playerTRC, playerBLC, playerBRC)
        //        merchantStashInvLocation(stashTLC, stashTRC, stashBLC, stashBRC)
}

func ItemDescriptionLocation() (image.Image, error) {
        mx, _ := robotgo.Location()
        mx = mx - int(float32(MonitorWidth)*0.25)
        mw := int(float32(MonitorWidth) * 0.45)
        if mw+mx > MonitorWidth+XOffset {
                mw = MonitorWidth + XOffset - mx
        }

        captureImg := robotgo.CaptureImg(mx, 0, mw, MonitorHeight)
        img, _ := gocv.ImageToMatRGB(captureImg)
        defer img.Close()
        gocv.IMWrite("./internal/resources/images/meta/precorneritemdescription-test.png", img)

        path := "./internal/resources/images/corners/nobackground/"
        trc := gocv.IMRead(path+"itemCorner-TopRight.png", gocv.IMReadColor)
        blc := gocv.IMRead(path+"itemCorner-BottomLeft.png", gocv.IMReadColor)
        defer trc.Close()
        defer blc.Close()
        gocv.CvtColor(img, &img, gocv.ColorBGRToGray)
        gocv.CvtColor(trc, &trc, gocv.ColorBGRToGray)
        gocv.CvtColor(blc, &blc, gocv.ColorBGRToGray)

        var threshold float32 = 0.97
        result := gocv.NewMat()
        defer result.Close()
        log.Println("item description")
        log.Println("----------------")

        gocv.MatchTemplate(img, trc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        trcmatch := GetMatchesFromTemplateMatchResult(result, threshold)
        log.Println("top right: ", trcmatch)

        gocv.MatchTemplate(img, blc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        blcmatch := GetMatchesFromTemplateMatchResult(result, threshold)
        log.Println("bottom left: ", blcmatch)

        if len(blcmatch) == 0 || len(trcmatch) == 0 {
                return nil, errors.New("could not find corners")
        }
        w := trcmatch[0].X - blcmatch[0].X + 5
        h := blcmatch[0].Y - trcmatch[0].Y + 5
        x := blcmatch[0].X + mx
        y := trcmatch[0].Y + YOffset
        log.Printf("X: %d, Y: %d, W: %d, H: %d", x, y, w, h)
        ci := robotgo.CaptureImg(
                x,
                y,
                w,
                h)
        i, _ := gocv.ImageToMatRGB(ci)
        defer i.Close()
        gocv.IMWrite("./internal/resources/images/meta/itemdescription-test.png", i)

        return ci, nil
}

func stashInvLocation(tlc, trc, blc, brc gocv.Mat) {
        captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
        img, _ := gocv.ImageToMatRGB(captureImg)
        defer img.Close()

        log.Println("stash inv")
        log.Println("---------")

        result := gocv.NewMat()
        defer result.Close()
        gocv.MatchTemplate(img, tlc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())

        matches := GetMatchesFromTemplateMatchResult(result, 0.9)
        matches[0].X = matches[0].X + 6
        matches[0].Y = matches[0].Y + 4

        log.Println(matches)
}
func stashPlayerInvLocation(tlc, trc, blc, brc gocv.Mat) {
        captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
        img, _ := gocv.ImageToMatRGB(captureImg)
        defer img.Close()

        log.Println("stash player inv")
        log.Println("----------------")

        var threshold float32 = 0.99
        result := gocv.NewMat()
        defer result.Close()
        var match []robotgo.Point

        gocv.MatchTemplate(img, tlc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        tlcmatch := GetMatchesFromTemplateMatchResult(result, threshold)
        if len(tlcmatch) == 1 {
                tlcmatch[0].X = tlcmatch[0].X + 0
                tlcmatch[0].Y = tlcmatch[0].Y + 0
        }
        log.Println("top left: ", tlcmatch)

        gocv.MatchTemplate(img, trc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        match = GetMatchesFromTemplateMatchResult(result, threshold)
        if len(match) == 1 {
                match[0].X = match[0].X + 0
                match[0].Y = match[0].Y + 0
        }
        log.Println("top right: ", match)

        gocv.MatchTemplate(img, blc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        match = GetMatchesFromTemplateMatchResult(result, threshold)
        if len(match) == 1 {
                match[0].X = match[0].X + 0
                match[0].Y = match[0].Y + 0
        }
        log.Println("bottom left: ", match)

        gocv.MatchTemplate(img, brc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
        brcmatch := GetMatchesFromTemplateMatchResult(result, threshold)
        if len(brcmatch) == 1 {
                brcmatch[0].X = brcmatch[0].X + 5
                brcmatch[0].Y = brcmatch[0].Y + 5
        }
        log.Println("bottom right: ", brcmatch)

        if len(brcmatch) == 0 || len(match) == 0 || len(tlcmatch) == 0 {
                return
        }

        ci := robotgo.CaptureImg(
                tlcmatch[0].X+XOffset,
                tlcmatch[0].Y+YOffset,
                brcmatch[0].X-tlcmatch[0].X,
                brcmatch[0].Y-tlcmatch[0].Y)
        i, _ := gocv.ImageToMatRGB(ci)
        defer i.Close()
        gocv.IMWrite("internal/resources/images/meta/stashplayerinv-test.png", i)
}

func merchantInvLocation(tlc, trc, blc, brc gocv.Mat) {

}

func merchantPlayerInvLocation(tlc, trc, blc, brc gocv.Mat) {

}

func merchantStashInvLocation(tlc, trc, blc, brc gocv.Mat) {

}

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
        sb := structs.GetSearchBox("Player Inventory Merchant")
        w := sb.RightX - sb.LeftX
        h := sb.BottomY - sb.TopY
        capture := robotgo.CaptureScreen(sb.LeftX+XOffset, sb.TopY+YOffset, w, h)
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

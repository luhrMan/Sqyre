package data

import (
	"Squire/internal/utils"
	"fmt"
	"image"
	"image/color"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/dialog"
	"github.com/go-vgo/robotgo"
	"gocv.io/x/gocv"
)

func CalibrateInventorySearchboxes(c Coordinates) {
	var (
		stashTLC         = gocv.IMRead(CalibrationImagesPath+"stashCorner-TopLeft"+PNG, gocv.IMReadColor)
		stashBRC         = gocv.IMRead(CalibrationImagesPath+"stashCorner-BottomRight"+PNG, gocv.IMReadColor)
		playerTLC        = gocv.IMRead(CalibrationImagesPath+"playerCorner-TopLeft"+PNG, gocv.IMReadColor)
		playerBRC        = gocv.IMRead(CalibrationImagesPath+"playerCorner-BottomRight"+PNG, gocv.IMReadColor)
		stashTabActive   = gocv.IMRead(CalibrationImagesPath+"stashTabActive"+PNG, gocv.IMReadColor)
		stashTabInactive = gocv.IMRead(CalibrationImagesPath+"stashTabInactive"+PNG, gocv.IMReadColor)
	)
	TopMenuTabLocations(c)

	robotgo.Move(c.Points[StashScr].X+XOffset, c.Points[StashScr].Y+YOffset)
	robotgo.MilliSleep(200)
	robotgo.Click()
	robotgo.MilliSleep(200)

	searchAreaInventoryAdd := func(sa SearchArea, name string) {
		c.AddSearchArea(sa)
		ci := robotgo.CaptureImg(
			sa.LeftX+XOffset,
			sa.TopY+YOffset,
			sa.RightX-sa.LeftX,
			sa.BottomY-sa.TopY)
		i, err := gocv.ImageToMatRGB(ci)
		if err != nil {
			fmt.Errorf("failed to capture "+name, err)
		}
		defer i.Close()
		gocv.IMWrite(MetaImagesPath+name+"-"+Empty+PNG, i)
		gocv.IMWrite(MaskImagesPath+DarkAndDarker+"/"+name+"-"+Empty+PNG, i)
	}

	sa, err := SearchAreaLocation(playerTLC, playerBRC, StashScrPlayerInv, 0.99)
	if err != nil {
		log.Println(err)
	} else {
		searchAreaInventoryAdd(sa, StashScrPlayerInv)
	}
	sa, err = SearchAreaLocation(stashTLC, stashBRC, StashScrStashInv, 0.9)
	if err != nil {
		log.Println(err)
	} else {
		searchAreaInventoryAdd(sa, StashScrStashInv)
	}
	StashInvTabsLocation(stashTabActive, stashTabInactive, StashScr)

	err = MerchantPortraitsLocation(c)
	if err != nil {
		log.Println(err)
		dialog.ShowInformation("Merchant Portrait Calibration Failed", err.Error(), fyne.CurrentApp().Driver().AllWindows()[0])
	} else {
		robotgo.Move(c.Points["Alchemist"].X, c.Points["Alchemist"].X)
		robotgo.Click()
		robotgo.MilliSleep(200)

		sa, err = SearchAreaLocation(playerTLC, playerBRC, MerchantsScrPlayerInv, 0.9)
		if err != nil {
			log.Println(err)
		} else {
			searchAreaInventoryAdd(sa, MerchantsScrPlayerInv)
		}
		StashInvTabsLocation(stashTabActive, stashTabInactive, MerchantsScr)

		sa, err = SearchAreaLocation(stashTLC, stashBRC, MerchantsScrStashInv, 0.9)
		if err != nil {
			log.Println(err)
		} else {
			searchAreaInventoryAdd(sa, MerchantsScrStashInv)
		}

		//add merchant inventory location search here
	}
}

func ItemDescriptionLocation() (image.Image, error) {
	mx, _ := robotgo.Location()
	mx = mx - int(float32(MonitorWidth)*0.25)
	mw := int(float32(MonitorWidth) * 0.50)
	if mw+mx > MonitorWidth+XOffset {
		mw = MonitorWidth + XOffset - mx
	}

	captureImg := robotgo.CaptureImg(mx, 0, mw, MonitorHeight)
	img, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		log.Println("Could not convert Image to MatRGB:", err)
	}
	defer img.Close()
	gocv.IMWrite(MetaImagesPath+"precorneritemdescription"+PNG, img)

	trc := gocv.IMRead(CalibrationImagesPath+"itemCorner-TopRight"+PNG, gocv.IMReadColor)
	blc := gocv.IMRead(CalibrationImagesPath+"itemCorner-BottomLeft"+PNG, gocv.IMReadColor)
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

	trcmatch, err := findCornerCoordinates(img, trc, result, threshold, true)
	if err != nil {
		return nil, fmt.Errorf("could not find item description | Top Right Corner")
	}
	log.Println("top right: ", trcmatch)

	blcmatch, err := findCornerCoordinates(img, blc, result, threshold, false)
	if err != nil {
		return nil, fmt.Errorf("could not find item description | Bottom Left Corner")
	}
	log.Println("bottom left: ", blcmatch)

	w := trcmatch[0].X - blcmatch[0].X + 20
	h := blcmatch[0].Y - trcmatch[0].Y + 20
	x := blcmatch[0].X + mx
	y := trcmatch[0].Y + YOffset
	log.Printf("X: %d, Y: %d, W: %d, H: %d", x, y, w, h)
	ci := robotgo.CaptureImg(
		x,
		y,
		w,
		h)
	i, err := gocv.ImageToMatRGB(ci)
	if err != nil {
		log.Println("Could not convert Image to MatRGB:", err)
	}
	defer i.Close()
	gocv.IMWrite(MetaImagesPath+"itemdescription"+PNG, i)

	return ci, nil
}

func findCornerCoordinates(img, corner, result gocv.Mat, threshold float32, resultOffset bool) ([]robotgo.Point, error) {
	gocv.MatchTemplate(img, corner, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
	match := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)

	switch {
	case len(match) == 1:
		switch resultOffset {
		case true:
			match[0].X = match[0].X + corner.Cols() //resultOffset
			match[0].Y = match[0].Y + corner.Rows() //resultOffset
		case false:
			match[0].X = match[0].X //resultOffset
			match[0].Y = match[0].Y
		}
		return match, nil

	case len(match) > 1:
		return []robotgo.Point{}, fmt.Errorf("found too many matches of corner")
	case len(match) == 0:
		return []robotgo.Point{}, fmt.Errorf("no matches found of corner")
	}

	return nil, fmt.Errorf("unknown error has occured")
}

func SearchAreaLocation(tlc, brc gocv.Mat, name string, threshold float32) (SearchArea, error) {
	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	img, err := gocv.ImageToMatRGB(captureImg)
	if err != nil {
		log.Println(fmt.Errorf("could not convert Image to MatRGB:", err))
	}
	defer img.Close()

	log.Println(name)
	log.Println("------------------------")

	result := gocv.NewMat()
	defer result.Close()

	tlcmatch, err := findCornerCoordinates(img, tlc, result, threshold, false)
	if err != nil {
		return SearchArea{}, fmt.Errorf("could not find " + name + " | Top Left Corner")
	}
	log.Println("top left | "+name, tlcmatch)

	brcmatch, err := findCornerCoordinates(img, brc, result, threshold, true)
	if err != nil {
		return SearchArea{}, fmt.Errorf("could not find " + name + " | Bottom Right Corner")
	}
	log.Println("bottom right | "+name, brcmatch)

	return SearchArea{
		Name:    name,
		LeftX:   tlcmatch[0].X,
		TopY:    tlcmatch[0].Y,
		RightX:  brcmatch[0].X,
		BottomY: brcmatch[0].Y,
	}, nil
}

func StashInvTabsLocation(active, inactive gocv.Mat, topMenuTab string) {
	var prefs = fyne.CurrentApp().Preferences()

	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()

	m := gocv.IMRead(MaskImagesPath+DarkAndDarker+"/"+"stashTabs mask"+PNG, gocv.IMReadColor)

	log.Println(topMenuTab + " stash tabs")
	log.Println("------------------------")

	result := gocv.NewMat()
	defer result.Close()
	var threshold float32 = 0.95

	gocv.MatchTemplate(img, active, &result, 5, m)
	matches := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)
	gocv.MatchTemplate(img, inactive, &result, 5, m)
	matches2 := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)

	matches = append(matches, matches2...)
	matches = utils.SortPoints(matches, "TopLeftToBottomRight")

	sbm := JsonPointMap()

	for i, m := range matches {
		tabName := topMenuTab + "-stashtab" + strconv.Itoa(i+1)
		prefs.SetIntList(tabName, []int{m.X, m.Y})
		sbm[tabName] = Point{Name: tabName, X: prefs.IntList(tabName)[0], Y: prefs.IntList(tabName)[1]}
		robotgo.Move(m.X+XOffset, m.Y+YOffset)
		robotgo.MilliSleep(200)
	}
}

func stashInventorySlots() {
	img := gocv.IMRead(ImagesPath+StashInv+Empty+PNG, gocv.IMReadColor)
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
	window := gocv.NewWindow(Inv)
	defer window.Close()
	window.IMShow(img)
	gocv.WaitKey(0)
}

func merchantPlayerInventorySlots() {
	sb := GetSearchArea(MerchantsScrPlayerInv)
	w := sb.RightX - sb.LeftX
	h := sb.BottomY - sb.TopY
	capture := robotgo.CaptureScreen(sb.LeftX+XOffset, sb.TopY+YOffset, w, h)
	robotgo.SaveJpeg(robotgo.ToImage(capture), ImagesPath+"search-area"+PNG)
	defer robotgo.FreeBitmap(capture)

	img := gocv.IMRead(ImagesPath+"search-area"+PNG, gocv.IMReadColor)
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

func MerchantPortraitsLocation(c Coordinates) error {
	if _, ok := c.Points[MerchantsScr]; !ok {
		err := "cannot find Merchants-screen. please calibrate the Top Menu"
		return fmt.Errorf(err)
	}
	robotgo.Move(c.Points[MerchantsScr].X+XOffset, c.Points[MerchantsScr].Y+YOffset)
	robotgo.MilliSleep(200)
	robotgo.Click()
	robotgo.MilliSleep(200)
	merchants := []string{
		"Alchemist",
		"Armourer",
		"Fortune Teller",
		"Goblin Merchant",
		"Goldsmith",
		"Leathersmith",
		"Nicholas",
		"Squire",
		"Surgeon",
		"Tailor",
		"Tavern Master",
		"The Collector",
		"Treasurer",
		"Weaponsmith",
		"Woodsman",
	}
	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	i, _ := gocv.ImageToMatRGB(captureImg)
	imgDraw := i.Clone()
	gocv.CvtColor(i, &i, gocv.ColorRGBToGray)
	t := gocv.IMRead(CalibrationImagesPath+"merchantPortraitTop"+PNG, gocv.IMReadGrayScale)
	m := gocv.IMRead(MaskImagesPath+DarkAndDarker+"/"+"merchantPortraitTop mask"+PNG, gocv.IMReadGrayScale)
	result := gocv.NewMat()
	defer i.Close()
	defer imgDraw.Close()
	defer t.Close()
	defer m.Close()
	defer result.Close()

	gocv.MatchTemplate(i, t, &result, 5, m)
	matches := utils.GetMatchesFromTemplateMatchResult(result, 0.9, 10)

	utils.DrawFoundMatches(matches, t.Cols(), t.Rows(), imgDraw, "")
	gocv.IMWrite(MetaImagesPath+"merchantPortraitsLocation-foundMerchants"+PNG, imgDraw)

	for _, match := range matches {
		h := t.Rows() / 2
		img := robotgo.CaptureImg(match.X+XOffset, match.Y+YOffset+h, t.Cols(), h)
		img = utils.ImageToMatToImagePreprocess(img, true, true, true, true, utils.PreprocessOptions{MinThreshold: 150})
		_, foundText := utils.CheckImageForText(img)

		log.Printf("FOUND TEXT: %v", foundText)
		if slices.Contains(merchants, foundText) {
			log.Printf("Saving point: %s, [%d, %d]", foundText, match.X, match.Y)
			c.AddPoint(Point{Name: foundText, X: match.X, Y: match.Y})
		}
		robotgo.Move(match.X+XOffset, match.Y+YOffset)
		robotgo.MilliSleep(200)
	}
	return nil
}
func TopMenuTabLocations(c Coordinates) {
	topMenuTabs := []string{
		"Play",
		"Leaderboard",
		"Religion",
		"Class",
		"Stash",
		"Merchants",
		"Trade",
		"Gathering Hall",
		"Customize",
		"Shop",
	}
	x := int((float32(MonitorWidth) - (float32(MonitorWidth) * 0.25)) * 0.11)
	y := int(float32(MonitorHeight) * 0.04)
	nx := int(float32(MonitorWidth) * 0.125)
	for _, t := range topMenuTabs {
		name := t + "-screen"
		c.AddPoint(Point{Name: name, X: nx, Y: y})
		nx += x
		robotgo.MilliSleep(200)
		robotgo.Move(c.Points[name].X+XOffset, c.Points[name].X+YOffset)
		log.Printf(name+": %d %d", c.Points[name].X, c.Points[name].Y)
	}
}

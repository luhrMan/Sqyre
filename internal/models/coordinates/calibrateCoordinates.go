package coordinates

// import (
// 	"Squire/internal/config"
// 	"Squire/internal/utils"
// 	"fmt"
// 	"image"
// 	"image/color"
// 	"log"
// 	"slices"
// 	"strconv"
// 	"strings"

// 	"fyne.io/fyne/v2"
// 	"fyne.io/fyne/v2/dialog"
// 	"github.com/go-vgo/robotgo"
// 	"gocv.io/x/gocv"
// )

// func CalibrateInventorySearchboxes(c *Coordinates) {
// 	var (
// 		p                = config.UpDir + config.UpDir + config.CalibrationImagesPath
// 		stashTLC         = gocv.IMRead(p+"stashCorner-TopLeft"+config.PNG, gocv.IMReadColor)
// 		stashBRC         = gocv.IMRead(p+"stashCorner-BottomRight"+config.PNG, gocv.IMReadColor)
// 		playerTLC        = gocv.IMRead(p+"playerCorner-TopLeft"+config.PNG, gocv.IMReadColor)
// 		playerBRC        = gocv.IMRead(p+"playerCorner-BottomRight"+config.PNG, gocv.IMReadColor)
// 		stashTabActive   = gocv.IMRead(p+"stashTabActive"+config.PNG, gocv.IMReadColor)
// 		stashTabInactive = gocv.IMRead(p+"stashTabInactive"+config.PNG, gocv.IMReadColor)
// 	)
// 	if stashTLC.Empty() ||
// 		stashBRC.Empty() ||
// 		playerTLC.Empty() ||
// 		playerBRC.Empty() ||
// 		stashTabActive.Empty() ||
// 		stashTabInactive.Empty() {
// 		log.Println("Could not read a calibration image")
// 		return
// 	}
// 	// CalibrateTopMenuTabLocations(c)

// 	robotgo.Move(c.Points[config.StashScr].X+config.XOffset, c.Points[config.StashScr].Y+config.YOffset)
// 	robotgo.MilliSleep(200)
// 	robotgo.Click()
// 	robotgo.MilliSleep(200)

// 	searchAreaInventoryAdd := func(sa SearchArea, name string) {
// 		c.AddSearchArea(sa)
// 		ci := robotgo.CaptureImg(
// 			sa.LeftX+config.XOffset,
// 			sa.TopY+config.YOffset,
// 			sa.RightX-sa.LeftX,
// 			sa.BottomY-sa.TopY)
// 		i, err := gocv.ImageToMatRGB(ci)
// 		if err != nil {
// 			fmt.Errorf("failed to capture "+name, err)
// 		}
// 		defer i.Close()
// 		gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+name+"-"+config.Empty+config.PNG, i)
// 		gocv.IMWrite(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+name+"-"+config.Empty+config.PNG, i)
// 	}

// 	sa, err := SearchAreaLocation(playerTLC, playerBRC, config.StashScrPlayerInv, 0.99)
// 	if err != nil {
// 		log.Println(err)
// 	} else {
// 		searchAreaInventoryAdd(sa, config.StashScrPlayerInv)
// 		robotgo.Move(c.SearchAreas[config.StashScrPlayerInv].LeftX+config.XOffset, c.SearchAreas[config.StashScrPlayerInv].TopY+config.YOffset)
// 		robotgo.MilliSleep(400)
// 		robotgo.Move(c.SearchAreas[config.StashScrPlayerInv].RightX+config.XOffset, c.SearchAreas[config.StashScrPlayerInv].BottomY+config.YOffset)
// 	}
// 	sa, err = SearchAreaLocation(stashTLC, stashBRC, config.StashScrStashInv, 0.9)
// 	if err != nil {
// 		log.Println(err)
// 	} else {
// 		searchAreaInventoryAdd(sa, config.StashScrStashInv)
// 		robotgo.Move(c.SearchAreas[config.StashScrStashInv].LeftX+config.XOffset, c.SearchAreas[config.StashScrStashInv].TopY+config.YOffset)
// 		robotgo.MilliSleep(400)
// 		robotgo.Move(c.SearchAreas[config.StashScrStashInv].RightX+config.XOffset, c.SearchAreas[config.StashScrStashInv].BottomY+config.YOffset)

// 	}
// 	StashInvTabsLocation(stashTabActive, stashTabInactive, config.StashScr, c)

// 	err = MerchantPortraitsLocation(c)
// 	if err != nil {
// 		log.Println(err)
// 		dialog.ShowInformation("Merchant Portrait Calibration Failed", err.Error(), fyne.CurrentApp().Driver().AllWindows()[0])
// 	} else {
// 		robotgo.Move(c.Points["alchemist"].X+config.XOffset, c.Points["alchemist"].Y+config.YOffset)
// 		robotgo.Click()
// 		robotgo.MilliSleep(200)

// 		sa, err = SearchAreaLocation(playerTLC, playerBRC, config.MerchantsScrPlayerInv, 0.9)
// 		if err != nil {
// 			log.Println(err)
// 		} else {
// 			searchAreaInventoryAdd(sa, config.MerchantsScrPlayerInv)
// 		}
// 		StashInvTabsLocation(stashTabActive, stashTabInactive, config.MerchantsScr, c)

// 		sa, err = SearchAreaLocation(stashTLC, stashBRC, config.MerchantsScrStashInv, 0.9)
// 		if err != nil {
// 			log.Println(err)
// 		} else {
// 			searchAreaInventoryAdd(sa, config.MerchantsScrStashInv)
// 		}

// 		//add merchant inventory location search here
// 	}
// 	log.Println("CALIBRATION COMPLETE")
// }

// func SearchAreaLocation(tlc, brc gocv.Mat, name string, threshold float32) (SearchArea, error) {
// 	captureImg := robotgo.CaptureImg(config.XOffset, config.YOffset, config.MonitorWidth, config.MonitorHeight)
// 	img, err := gocv.ImageToMatRGB(captureImg)
// 	if err != nil {
// 		log.Println(fmt.Errorf("could not convert Image to MatRGB:", err))
// 	}
// 	defer img.Close()

// 	log.Println(name)
// 	log.Println("------------------------")

// 	result := gocv.NewMat()
// 	defer result.Close()

// 	tlcmatch, err := findCornerCoordinates(img, tlc, result, threshold, false)
// 	if err != nil {
// 		return SearchArea{}, fmt.Errorf("could not find " + name + " | Top Left Corner")
// 	}
// 	log.Println("top left | "+name, tlcmatch)

// 	brcmatch, err := findCornerCoordinates(img, brc, result, threshold, true)
// 	if err != nil {
// 		return SearchArea{}, fmt.Errorf("could not find " + name + " | Bottom Right Corner")
// 	}
// 	log.Println("bottom right | "+name, brcmatch)

// 	return SearchArea{
// 		Name:    name,
// 		LeftX:   tlcmatch[0].X,
// 		TopY:    tlcmatch[0].Y,
// 		RightX:  brcmatch[0].X,
// 		BottomY: brcmatch[0].Y,
// 	}, nil
// }

// func StashInvTabsLocation(active, inactive gocv.Mat, topMenuTab string, c *Coordinates) {
// 	captureImg := robotgo.CaptureImg(config.XOffset, config.YOffset, config.MonitorWidth, config.MonitorHeight)
// 	img, _ := gocv.ImageToMatRGB(captureImg)
// 	defer img.Close()

// 	m := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"stashTabs mask"+config.PNG, gocv.IMReadColor)

// 	log.Println(topMenuTab + " stash tabs")
// 	log.Println("------------------------")

// 	result := gocv.NewMat()
// 	defer result.Close()
// 	var threshold float32 = 0.95

// 	gocv.MatchTemplate(img, active, &result, 5, m)
// 	matches := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)
// 	gocv.MatchTemplate(img, inactive, &result, 5, m)
// 	matches2 := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)

// 	matches = append(matches, matches2...)
// 	matches = utils.SortPoints(matches, "TopLeftToBottomRight")

// 	points := c.Points
// 	for i, m := range matches {
// 		tabName := topMenuTab + "-stashtab" + strconv.Itoa(i+1)
// 		points[tabName] = Point{Name: tabName, X: m.X, Y: m.Y}
// 		robotgo.Move(m.X+config.XOffset, m.Y+config.YOffset)
// 		robotgo.MilliSleep(200)
// 	}
// }

// func stashInventorySlots() {
// 	img := gocv.IMRead(config.UpDir+config.UpDir+config.ImagesPath+config.StashInv+config.Empty+config.PNG, gocv.IMReadColor)
// 	if img.Empty() {
// 		fmt.Println("Error reading main image")
// 	}
// 	defer img.Close()
// 	invRows := 20
// 	invCols := 12
// 	xSize := img.Cols() / invCols
// 	ySize := img.Rows() / invRows
// 	var invSlots []image.Rectangle
// 	//box := image.Rectangle{image.Point{0,0}, image.Point{img.Rows() / 12, img.Cols() / 24}}
// 	for r := 0; r < invCols; r++ {
// 		for c := 0; c < invRows; c++ {
// 			invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
// 		}
// 	}
// 	for _, rect := range invSlots {
// 		windowSlot := gocv.NewWindow("slot")
// 		defer windowSlot.Close()
// 		windowSlot.IMShow(img.Region(rect))
// 		gocv.WaitKey(0)
// 		gocv.Rectangle(&img, rect, color.RGBA{R: 255, A: 255}, 2)
// 	}
// 	window := gocv.NewWindow(config.Inv)
// 	defer window.Close()
// 	window.IMShow(img)
// 	gocv.WaitKey(0)
// }

// func merchantPlayerInventorySlots(c *Coordinates) {

// 	sa := c.SearchAreas[config.MerchantsScrPlayerInv] // GetSearchArea(config.MerchantsScrPlayerInv)
// 	w := sa.RightX - sa.LeftX
// 	h := sa.BottomY - sa.TopY
// 	capture := robotgo.CaptureScreen(sa.LeftX+config.XOffset, sa.TopY+config.YOffset, w, h)
// 	robotgo.SaveJpeg(robotgo.ToImage(capture), config.UpDir+config.UpDir+config.ImagesPath+"search-area"+config.PNG)
// 	defer robotgo.FreeBitmap(capture)

// 	img := gocv.IMRead(config.UpDir+config.UpDir+config.ImagesPath+"search-area"+config.PNG, gocv.IMReadColor)
// 	if img.Empty() {
// 		fmt.Println("Error reading main image")
// 	}
// 	defer img.Close()

// 	invRows := 5
// 	invCols := 10
// 	xSize := img.Cols() / invCols
// 	ySize := img.Rows() / invRows

// 	//	var invSlotMats []gocv.Mat
// 	//	for i, s := range invSlots {
// 	//		invSlotMats = append(invSlotMats, img.Region(s))
// 	//		defer invSlotMats[i].Close()
// 	//	}

// 	var invSlots []image.Rectangle
// 	for r := 0; r < invCols; r++ {
// 		for c := 0; c < invRows; c++ {
// 			invSlots = append(invSlots, image.Rect(xSize*r, ySize*c, xSize+xSize*r, ySize+ySize*c))
// 		}
// 	}
// }

// func MerchantPortraitsLocation(c *Coordinates) error {
// 	if _, ok := c.Points[config.MerchantsScr]; !ok {
// 		err := "cannot find Merchants-screen. please calibrate the Top Menu"
// 		return fmt.Errorf(err)
// 	}
// 	robotgo.Move(c.Points[config.MerchantsScr].X+config.XOffset, c.Points[config.MerchantsScr].Y+config.YOffset)
// 	robotgo.MilliSleep(200)
// 	robotgo.Click()
// 	robotgo.MilliSleep(200)
// 	merchants := []string{
// 		"Alchemist",
// 		"Armourer",
// 		"Fortune Teller",
// 		"Goblin Merchant",
// 		"Goldsmith",
// 		"Leathersmith",
// 		"Nicholas",
// 		"Squire",
// 		"Surgeon",
// 		"Tailor",
// 		"Tavern Master",
// 		"The Collector",
// 		"Treasurer",
// 		"Valentine",
// 		"Weaponsmith",
// 		"Woodsman",
// 	}
// 	captureImg := robotgo.CaptureImg(config.XOffset, config.YOffset, config.MonitorWidth, config.MonitorHeight)
// 	i, _ := gocv.ImageToMatRGB(captureImg)
// 	imgDraw := i.Clone()
// 	gocv.CvtColor(i, &i, gocv.ColorRGBToGray)
// 	t := gocv.IMRead(config.UpDir+config.UpDir+config.CalibrationImagesPath+"merchantPortraitTop"+config.PNG, gocv.IMReadGrayScale)
// 	m := gocv.IMRead(config.UpDir+config.UpDir+config.MaskImagesPath+config.DarkAndDarker+"/"+"merchantPortraitTop mask"+config.PNG, gocv.IMReadGrayScale)
// 	result := gocv.NewMat()
// 	defer i.Close()
// 	defer imgDraw.Close()
// 	defer t.Close()
// 	defer m.Close()
// 	defer result.Close()

// 	gocv.MatchTemplate(i, t, &result, 5, m)
// 	matches := utils.GetMatchesFromTemplateMatchResult(result, 0.9, 10)

// 	utils.DrawFoundMatches(matches, t.Cols(), t.Rows(), imgDraw, "")
// 	gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"merchantPortraitsLocation/foundMerchants"+config.PNG, imgDraw)

// 	for _, match := range matches {
// 		h := t.Rows() / 2
// 		img := robotgo.CaptureImg(match.X+config.XOffset, match.Y+config.YOffset+h, t.Cols(), h)
// 		img = utils.ImageToMatToImagePreprocess(img, true, true, true, true, utils.PreprocessOptions{MinThreshold: 160})
// 		mat, err := gocv.ImageToMatRGB(img)
// 		if err != nil {
// 			log.Println(err)
// 			continue
// 		}
// 		err, foundText := utils.CheckImageForText(img)
// 		if err != nil {
// 			log.Println(err)
// 			continue
// 		}
// 		gocv.IMWrite(config.UpDir+config.UpDir+config.MetaImagesPath+"merchantPortraitsLocation/"+foundText+config.PNG, mat)

// 		log.Printf("FOUND TEXT: %v", foundText)
// 		if slices.Contains(merchants, foundText) {
// 			foundText = strings.ToLower(foundText)
// 			log.Printf("Saving point: %s, [%d, %d]", foundText, match.X+(t.Cols()/2), match.Y+h)
// 			c.AddPoint(Point{Name: foundText, X: match.X + (t.Cols() / 2), Y: match.Y + h})
// 			robotgo.Move(c.Points[foundText].X+config.XOffset, c.Points[foundText].Y+config.YOffset)
// 		}
// 		robotgo.MilliSleep(200)
// 	}
// 	return nil
// }
// func CalibrateTopMenuTabLocations(c *Coordinates) {
// 	topMenuTabs := []string{
// 		"Play",
// 		"Leaderboard",
// 		"Religion",
// 		"Class",
// 		"Stash",
// 		"Merchants",
// 		"Trade",
// 		"Gathering Hall",
// 		"Customize",
// 		"Shop",
// 	}
// 	x := int((float32(config.MonitorWidth) - (float32(config.MonitorWidth) * 0.25)) * 0.11)
// 	y := int(float32(config.MonitorHeight) * 0.04)
// 	nx := int(float32(config.MonitorWidth) * 0.125)
// 	for _, t := range topMenuTabs {
// 		name := strings.ToLower(t) + "-screen"
// 		c.AddPoint(Point{Name: name, X: nx, Y: y})
// 		nx += x
// 		robotgo.MilliSleep(200)
// 		robotgo.Move(c.Points[name].X+config.XOffset, c.Points[name].Y+config.YOffset)
// 		log.Printf(name+": %d %d", c.Points[name].X, c.Points[name].Y)
// 	}
// }

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

func CalibrateInventorySearchboxes() {
	var prefs = fyne.CurrentApp().Preferences()

	path := imagesPath + "calibration/"
	sspi := "Stash-screen-player-inventory"
	sssi := "Stash-screen-stash-inventory"
	sbm := *GetSearchAreaMap()
	var (
		stashTLC         = gocv.IMRead(path+"stashCorner-TopLeft.png", gocv.IMReadColor)
		stashBRC         = gocv.IMRead(path+"stashCorner-BottomRight.png", gocv.IMReadColor)
		playerTLC        = gocv.IMRead(path+"playerCorner-TopLeft.png", gocv.IMReadColor)
		playerBRC        = gocv.IMRead(path+"playerCorner-BottomRight.png", gocv.IMReadColor)
		stashTabActive   = gocv.IMRead(path+"stashTabActive.png", gocv.IMReadColor)
		stashTabInactive = gocv.IMRead(path+"stashTabInactive.png", gocv.IMReadColor)
	)
	TopMenuTabLocations()
	robotgo.Move(prefs.IntList("Stash-screen")[0]+XOffset, prefs.IntList("Stash-screen")[1]+YOffset)
	robotgo.MilliSleep(200)
	robotgo.Click()
	robotgo.MilliSleep(200)
	PlayerInvLocation(playerTLC, playerBRC, "Stash-screen")
	sbm[sspi] = SearchArea{Name: sspi, LeftX: prefs.IntList(sspi)[0], TopY: prefs.IntList(sspi)[1], RightX: prefs.IntList(sspi)[2], BottomY: prefs.IntList(sspi)[3]}

	StashInvLocation(stashTLC, stashBRC, "Stash-screen")
	sbm[sssi] = SearchArea{Name: sssi, LeftX: prefs.IntList(sssi)[0], TopY: prefs.IntList(sssi)[1], RightX: prefs.IntList(sssi)[2], BottomY: prefs.IntList(sssi)[3]}
	StashInvTabsLocation(stashTabActive, stashTabInactive, "Stash-screen")

	MerchantPortraitsLocation()

	// robotgo.Move(prefs.IntList("Alchemist")[0], prefs.IntList("Alchemist")[1])
	// robotgo.Click()
	// robotgo.MilliSleep(200)
	// playerInvLocation(playerTLC, playerBRC, "merchant")
	// stashInvTabsLocation(stashTab, "merchant")
	// stashInvLocation(stashTLC, stashBRC, "merchant")

	// merchantInvLocation(stashTLC, stashTRC, stashBLC, stashBRC)
}

func ItemDescriptionLocation() (image.Image, error) {
	mx, _ := robotgo.Location()
	mx = mx - int(float32(MonitorWidth)*0.25)
	mw := int(float32(MonitorWidth) * 0.50)
	if mw+mx > MonitorWidth+XOffset {
		mw = MonitorWidth + XOffset - mx
	}

	captureImg := robotgo.CaptureImg(mx, 0, mw, MonitorHeight)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()
	gocv.IMWrite(imagesPath+"meta/precorneritemdescription-test.png", img)

	path := imagesPath + "calibration/"
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
	trcmatch := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)
	log.Println("top right: ", trcmatch)

	gocv.MatchTemplate(img, blc, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
	blcmatch := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)
	log.Println("bottom left: ", blcmatch)

	if len(blcmatch) == 0 || len(trcmatch) == 0 {
		log.Println("could not find corners")
		return nil, nil
	}
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
	i, _ := gocv.ImageToMatRGB(ci)
	defer i.Close()
	gocv.IMWrite(imagesPath+"meta/itemdescription-test.png", i)

	return ci, nil
}

func findCornerCoordinates(img, corner, result gocv.Mat, threshold float32, resultOffset bool) []robotgo.Point {
	gocv.MatchTemplate(img, corner, &result, gocv.TemplateMatchMode(5), gocv.NewMat())
	match := utils.GetMatchesFromTemplateMatchResult(result, threshold, 10)

	if len(match) == 1 {
		switch resultOffset {
		case true:
			match[0].X = match[0].X + corner.Cols() //resultOffset
			match[0].Y = match[0].Y + corner.Rows() //resultOffset
		case false:
			match[0].X = match[0].X //resultOffset
			match[0].Y = match[0].Y
		}
	}
	return match
}

func StashInvLocation(tlc, brc gocv.Mat, topMenuTab string) {
	var prefs = fyne.CurrentApp().Preferences()

	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()

	log.Println("tab " + topMenuTab + ": stash inv")
	log.Println("------------------------")

	result := gocv.NewMat()
	defer result.Close()
	var threshold float32 = 0.9

	tlcmatch := findCornerCoordinates(img, tlc, result, threshold, false)
	log.Println("top left: ", tlcmatch)

	brcmatch := findCornerCoordinates(img, brc, result, threshold, true)
	log.Println("bottom right: ", brcmatch)

	if len(tlcmatch) == 0 || len(brcmatch) == 0 {
		log.Println("could not find " + topMenuTab + " stash inventory corners")
		return
	}

	ci := robotgo.CaptureImg(
		tlcmatch[0].X+XOffset,
		tlcmatch[0].Y+YOffset,
		brcmatch[0].X-tlcmatch[0].X,
		brcmatch[0].Y-tlcmatch[0].Y)
	i, _ := gocv.ImageToMatRGB(ci)
	defer i.Close()
	gocv.IMWrite(imagesPath+"meta/"+topMenuTab+"-stash-test.png", i)
	gocv.IMWrite(masksPath+"Dark And Darker/"+topMenuTab+"-empty-stash-inventory.png", i)
	prefs.SetIntList(topMenuTab+"-stash-inventory", []int{tlcmatch[0].X, tlcmatch[0].Y, brcmatch[0].X, brcmatch[0].Y})
}

func StashInvTabsLocation(active, inactive gocv.Mat, topMenuTab string) {
	var prefs = fyne.CurrentApp().Preferences()

	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()

	m := gocv.IMRead(masksPath+"Dark And Darker/stashTabs mask.png", gocv.IMReadColor)

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

	sbm := *GetPointMap()

	for i, m := range matches {
		tabName := topMenuTab + "-stashtab" + strconv.Itoa(i+1)
		prefs.SetIntList(tabName, []int{m.X, m.Y})
		sbm[tabName] = Point{Name: tabName, X: prefs.IntList(tabName)[0], Y: prefs.IntList(tabName)[1]}
		robotgo.Move(m.X+XOffset, m.Y+YOffset)
		robotgo.MilliSleep(200)
	}
}

func PlayerInvLocation(tlc, brc gocv.Mat, topMenuTab string) {
	var prefs = fyne.CurrentApp().Preferences()

	captureImg := robotgo.CaptureImg(XOffset, YOffset, MonitorWidth, MonitorHeight)
	img, _ := gocv.ImageToMatRGB(captureImg)
	defer img.Close()

	log.Println(topMenuTab + " player inventory")
	log.Println("----------------")

	var threshold float32 = 0.99
	result := gocv.NewMat()
	defer result.Close()

	tlcmatch := findCornerCoordinates(img, tlc, result, threshold, false)
	log.Println("top left: ", tlcmatch)

	brcmatch := findCornerCoordinates(img, brc, result, threshold, true)
	log.Println("bottom right: ", brcmatch)

	if len(tlcmatch) == 0 || len(brcmatch) == 0 {
		log.Println("could not find " + topMenuTab + " player inventory corners")
		return
	}

	ci := robotgo.CaptureImg(
		tlcmatch[0].X+XOffset,
		tlcmatch[0].Y+YOffset,
		brcmatch[0].X-tlcmatch[0].X,
		brcmatch[0].Y-tlcmatch[0].Y)
	i, _ := gocv.ImageToMatRGB(ci)
	defer i.Close()
	gocv.IMWrite(imagesPath+"meta/"+topMenuTab+"-empty-player-inventory.png", i)
	gocv.IMWrite(masksPath+"Dark And Darker/"+topMenuTab+"-empty-player-inventory.png", i)

	prefs.SetIntList(topMenuTab+"-player-inventory", []int{tlcmatch[0].X, tlcmatch[0].Y, brcmatch[0].X, brcmatch[0].Y})
}

func merchantInvLocation(tlc, trc, blc, brc gocv.Mat) {

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
	sb := GetSearchArea("Player Inventory Merchant")
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

func MerchantPortraitsLocation() {
	var prefs = fyne.CurrentApp().Preferences()

	if len(prefs.IntList("Merchants-screen")) == 0 {
		dialog.ShowInformation("No Merchants-screen coordinates found", "Cannot find Merchants-screen. Please calibrate the Top Menu", fyne.CurrentApp().Driver().AllWindows()[0])
		return
	}
	robotgo.Move(prefs.IntList("Merchants-screen")[0]+XOffset, prefs.IntList("Merchants-screen")[1]+YOffset)
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
	t := gocv.IMRead(imagesPath+"calibration/merchantPortraitTop.png", gocv.IMReadGrayScale)
	m := gocv.IMRead(imagesPath+"masks/Dark And Darker/merchantPortraitTop mask.png", gocv.IMReadGrayScale)
	result := gocv.NewMat()
	defer i.Close()
	defer imgDraw.Close()
	defer t.Close()
	defer m.Close()
	defer result.Close()

	gocv.MatchTemplate(i, t, &result, 5, m)
	matches := utils.GetMatchesFromTemplateMatchResult(result, 0.9, 10)

	utils.DrawFoundMatches(matches, t.Cols(), t.Rows(), imgDraw, "")
	gocv.IMWrite(imagesPath+"meta/merchantPortraitsLocation-foundMerchants.png", imgDraw)

	for _, match := range matches {
		h := t.Rows() / 2
		img := robotgo.CaptureImg(match.X+XOffset, match.Y+YOffset+h, t.Cols(), h)
		img = utils.ImageToMatToImagePreprocess(img, true, true, true, true, utils.PreprocessOptions{MinThreshold: 150})
		_, foundText := utils.CheckImageForText(img)

		log.Printf("FOUND TEXT: %v", foundText)
		if slices.Contains(merchants, foundText) {
			log.Printf("Saving user preference location for: %s, [%d, %d]", foundText, match.X, match.Y)
			prefs.SetIntList(foundText, []int{match.X, match.Y})
		}
		robotgo.Move(match.X+XOffset, match.Y+YOffset)
		robotgo.MilliSleep(200)
	}
}
func TopMenuTabLocations() {
	var prefs = fyne.CurrentApp().Preferences()

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
	log.Println(MonitorWidth)
	x := int((float32(MonitorWidth) - (float32(MonitorWidth) * 0.25)) * 0.11)
	y := int(float32(MonitorHeight) * 0.04)
	nx := int(float32(MonitorWidth) * 0.125)
	for _, t := range topMenuTabs {
		name := t + "-screen"
		prefs.SetIntList(name, []int{nx, y})
		nx += x
		robotgo.MilliSleep(200)
		robotgo.Move(prefs.IntList(name)[0]+XOffset, prefs.IntList(name)[1]+YOffset)
		log.Printf(name+": %d %d", prefs.IntList(name)[0], prefs.IntList(name)[1])
	}
}

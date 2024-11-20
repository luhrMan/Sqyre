package main

import (
	"Dark-And-Darker/gui"
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"

	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/theme"
	"github.com/go-vgo/robotgo"

	"image"
	"image/color"

	"gocv.io/x/gocv"
)

type inventorySlot struct {
	Rectangle image.Rectangle
	AvgBGR    []float32
	AvgHSV    []float32
	AvgLAB    []float32
}

func main() {
	a := app.New()
	a.Settings().SetTheme(theme.DarkTheme())
	w := a.NewWindow("Squire")
	content := gui.LoadMainContent()
	icon, _ := fyne.LoadResourceFromPath("./images/Squire.png")
	w.SetIcon(icon)
	w.SetContent(content)
	w.ShowAndRun()
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

func testGoodFeatures() {
	img := gocv.IMRead("./images/empty-stash.jpeg", gocv.IMReadColor)
	if img.Empty() {
		fmt.Println("Error reading main image")
	}
	defer img.Close()
	gray := gocv.NewMat()
	defer gray.Close()
	corners := gocv.NewMat()
	defer corners.Close()

	gocv.CvtColor(img, &gray, gocv.ColorBGRToGray)
	//	gocv.GaussianBlur(gray, &gray, image.Point{9,9}, 0, 0, gocv.BorderDefault)

	//	c := gocv.FindContours(gray, gocv.RetrievalList, gocv.ChainApproxNone)
	//	log.Println(c.Size())
	//	gocv.DrawContours(&gray, c, -1, color.RGBA{R: 255}, 3)
	//gocv.Threshold(gray, &gray, 0, 50, 0 + gocv.ThresholdOtsu)
	//gocv.Canny(gray, &gray, 0, 200)
	//	cw := gocv.NewWindow("canny")
	//	defer cw.Close()
	//	cw.IMShow(gray)
	//	gocv.WaitKey(0)

	gocv.GoodFeaturesToTrack(gray, &corners, 100, 0.2, 100)

	// Extract the corner points from the corners Mat
	for r := 0; r < corners.Rows(); r++ {
		// Each row in 'corners' is a point (x, y)
		corner := corners.Row(r)
		x := corner.GetFloatAt(0, 0) // x-coordinate
		y := corner.GetFloatAt(0, 1) // y-coordinate
		log.Printf("Corner found at: (%d, %d)", int(x), int(y))

		// Draw a circle on the original image at the corner's location
		gocv.Circle(&img, image.Pt(int(x), int(y)), 3, color.RGBA{255, 0, 0, 0}, 2)
	}

	window := gocv.NewWindow("Good features")
	defer window.Close()
	window.IMShow(img)
	gocv.WaitKey(0)
	//gocv.Circle(&img, image.Pt(c, r), 2, color.RGBA{255, 0, 0, 0}, 2)
}

func testCornerDetection() {
	img := gocv.IMRead("./images/empty-stash.jpeg", gocv.IMReadColor)
	if img.Empty() {
		fmt.Println("Error reading main image")
	}
	defer img.Close()
	grayImg := gocv.NewMat()
	defer grayImg.Close()
	gocv.CvtColor(img, &grayImg, gocv.ColorBGRToGray)
	// Step 1: Compute the image gradients (Ix, Iy) using Sobel operator
	sobelX := gocv.NewMat()
	defer sobelX.Close()
	sobelY := gocv.NewMat()
	defer sobelY.Close()

	gocv.Sobel(grayImg, &sobelX, gocv.MatTypeCV32F, 1, 0, 9, 1, 0, gocv.BorderDefault)
	gocv.Sobel(grayImg, &sobelY, gocv.MatTypeCV32F, 0, 1, 9, 1, 0, gocv.BorderDefault)

	// Step 2: Compute the products of gradients (Ix^2, Iy^2, Ix*Iy)
	gradX2 := gocv.NewMat()
	defer gradX2.Close()
	gradY2 := gocv.NewMat()
	defer gradY2.Close()
	gradXY := gocv.NewMat()
	defer gradXY.Close()

	gocv.Multiply(sobelX, sobelX, &gradX2)
	gocv.Multiply(sobelY, sobelY, &gradY2)
	gocv.Multiply(sobelX, sobelY, &gradXY)

	// Step 3: Apply Gaussian filter to smooth these gradients (for noise reduction)
	// Here, we can use a simple Gaussian blur for smoothing
	gausNum := 5
	gocv.GaussianBlur(gradX2, &gradX2, image.Point{X: gausNum, Y: gausNum}, 0, 0, gocv.BorderDefault)
	gocv.GaussianBlur(gradY2, &gradY2, image.Point{X: gausNum, Y: gausNum}, 0, 0, gocv.BorderDefault)
	gocv.GaussianBlur(gradXY, &gradXY, image.Point{X: gausNum, Y: gausNum}, 0, 0, gocv.BorderDefault)

	// Step 4: Compute the Harris corner response (R)
	var k float32 = 0.05
	rows := gradX2.Rows()
	cols := gradX2.Cols()
	harrisResponse := gocv.NewMatWithSize(rows, cols, gocv.MatTypeCV32F)
	defer harrisResponse.Close()

	for r := 0; r < rows; r++ {
		for c := 0; c < cols; c++ {
			// Compute the determinant (det(M)) and trace (tr(M)) of the Harris matrix
			A := gradX2.GetFloatAt(r, c)
			B := gradY2.GetFloatAt(r, c)
			C := gradXY.GetFloatAt(r, c)

			detM := A*B - C*C
			traceM := A + B

			// Compute the Harris corner response
			R := detM - k*traceM*traceM
			harrisResponse.SetFloatAt(r, c, R)
		}
	}

	// Step 5: Normalize the Harris response for visualization
	normalizedResponse := gocv.NewMat()
	defer normalizedResponse.Close()
	gocv.Normalize(harrisResponse, &normalizedResponse, 0, 255, gocv.NormMinMax)

	nw := gocv.NewWindow("normalizedResponse")
	defer nw.Close()
	nw.IMShow(normalizedResponse)

	// Step 6: Detect corners based on a threshold
	for r := 0; r < normalizedResponse.Rows(); r++ {
		for c := 0; c < normalizedResponse.Cols(); c++ {
			// Threshold to detect strong corners
			if normalizedResponse.GetUCharAt(r, c) > 254 {
				gocv.Circle(&img, image.Pt(c, r), 1, color.RGBA{255, 0, 0, 0}, 2)
			}
		}
	}

	// Step 7: Display the result with corners marked
	window := gocv.NewWindow("Harris Corners")
	defer window.Close()
	window.IMShow(img)

	// Wait until the user presses a key
	gocv.WaitKey(0)
}

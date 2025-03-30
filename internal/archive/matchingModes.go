package archive

//func (a *ImageSearch) colorMatching(img, templateCut gocv.Mat, tolerance float32, target string, splitAreas []image.Rectangle) []robotgo.Point {
//	var colorMatchwg sync.WaitGroup
//	var matches []robotgo.Point
//	colorMatchResultsMutex := &sync.Mutex{}
//	emptyPoint := robotgo.Point{}
//
//	for _, s := range splitAreas { //for each split area, create a goroutine
//		colorMatchwg.Add(1)
//		go func(s image.Rectangle) {
//			defer colorMatchwg.Done()
//
//			var point robotgo.Point
//			point = checkHistogramMatch(img.Region(s), templateCut, tolerance, target)
//			if point != emptyPoint {
//				point = robotgo.Point{X: s.Min.X, Y: s.Min.Y}
//				colorMatchResultsMutex.Lock()
//				defer colorMatchResultsMutex.Unlock()
//				matches = append(matches, point)
//			}
//		}(s)
//	}
//	colorMatchwg.Wait()
//	return matches
//}
//
//func checkHistogramMatch(img, template gocv.Mat, tolerance float32, target string) robotgo.Point {
//	normType := gocv.NormMinMax
//	compType := gocv.HistCmpBhattacharya
//
//	getColorChannels := func(image gocv.Mat, colorModel gocv.ColorConversionCode) gocv.Mat {
//		colors := gocv.NewMat()
//		gocv.CvtColor(image, &colors, colorModel)
//		return colors
//	}
//
//	calculateColorModelSimilarities := func(img1, img2 gocv.Mat, bins int) []float32 {
//		comparisons := []float32{0, 0, 0}
//		img1Channels := gocv.Split(img1)
//		img2Channels := gocv.Split(img2)
//		for c := range img1Channels {
//			gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 256}, false)
//			gocv.Normalize(img1Channels[c], &img1Channels[c], 0, 1, normType)
//			gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 256}, false)
//			gocv.Normalize(img2Channels[c], &img2Channels[c], 0, 1, normType)
//			comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType)
//		}
//		return comparisons
//	}
//
//	calculateHSVColorModelSimilarities := func(img1, img2 gocv.Mat, bins int) []float32 {
//		comparisons := []float32{0, 0, 0}
//		img1Channels := gocv.Split(img1)
//		img2Channels := gocv.Split(img2)
//		for c := range img1Channels {
//			if c == 0 { //for hue, set range 0 - 180
//				gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 180}, false)
//				gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 180}, false)
//			} else {
//				gocv.CalcHist([]gocv.Mat{img1}, []int{c}, gocv.NewMat(), &img1Channels[c], []int{bins}, []float64{0, 256}, false)
//				gocv.CalcHist([]gocv.Mat{img2}, []int{c}, gocv.NewMat(), &img2Channels[c], []int{bins}, []float64{0, 256}, false)
//			}
//			gocv.Normalize(img1Channels[c], &img1Channels[c], 0, 1, normType)
//			gocv.Normalize(img2Channels[c], &img2Channels[c], 0, 1, normType)
//			if c == 1 { //for saturation, cut in half idk why it just was going high af
//				comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType) / 2
//			} else {
//				comparisons[c] = gocv.CompareHist(img1Channels[c], img2Channels[c], compType)
//			}
//		}
//		return comparisons
//	}
//
//	simBGR := calculateColorModelSimilarities(img, template, 64)
//
//	imgGray := getColorChannels(img, gocv.ColorBGRToGray)
//	templateGray := getColorChannels(template, gocv.ColorBGRToGray)
//	simGray := calculateColorModelSimilarities(imgGray, templateGray, 64)
//
//	imgHSV := getColorChannels(img, gocv.ColorBGRToHSV)
//	templateHSV := getColorChannels(template, gocv.ColorBGRToHSV)
//	simHSV := calculateHSVColorModelSimilarities(imgHSV, templateHSV, 64)
//
//	imgLAB := getColorChannels(img, gocv.ColorBGRToLab)
//	templateLAB := getColorChannels(template, gocv.ColorBGRToLab)
//	simLAB := calculateColorModelSimilarities(imgLAB, templateLAB, 64)
//
//	if
//	//	simGray < 0.06 &&
//	simBGR[0] < tolerance &&
//		simBGR[1] < tolerance &&
//		simBGR[2] < tolerance &&
//		simHSV[0] < tolerance &&
//		simHSV[1] < tolerance &&
//		simHSV[2] < tolerance &&
//		simLAB[0] < tolerance &&
//		simLAB[1] < tolerance &&
//		simLAB[2] < tolerance {
//		log.Printf("target: %v gray: %.4f\n"+
//			"l: %.4f || a: %.4f || b: %.4f\n"+
//			"hue: %.4f || sat: %.4f || val: %.4f\n"+
//			"blue: %.4f || green: %.4f || red: %.4f",
//			target, simGray[0],
//			simLAB[0], simLAB[1], simLAB[2],
//			simHSV[0], simHSV[1], simHSV[2],
//			simBGR[0], simBGR[1], simBGR[2])
//		return robotgo.Point{X: img.Size()[0], Y: img.Size()[1]}
//	} else {
//		return robotgo.Point{}
//	}
//}

//func (a *ImageSearch) featureMatching(img, template gocv.Mat, target string) []robotgo.Point {
//	//	sift := gocv.NewSIFT()
//	nFeatures := 0
//	nOctaveLayers := 5
//	contrastThreshold := 0.04
//	edgeThreshold := 300.0
//	sigma := 1.6
//	sift := gocv.NewSIFTWithParams(&nFeatures, &nOctaveLayers, &contrastThreshold, &edgeThreshold, &sigma)
//	defer sift.Close()
//
//	mask := gocv.NewMat()
//	defer mask.Close()
//
//	m := gocv.IMRead("./internal/resources/images/empty-stash.png", gocv.IMReadGrayScale)
//	defer m.Close()
//	diffed := gocv.NewMat()
//	defer diffed.Close()
//	threshM := gocv.NewMat()
//	defer threshM.Close()
//
//	grayI := gocv.NewMat()
//	threshI := gocv.NewMat()
//	bitwiseI := gocv.NewMat()
//	defer grayI.Close()
//	defer threshI.Close()
//	defer bitwiseI.Close()
//	gocv.CvtColor(img, &grayI, gocv.ColorBGRToGray)
//	gocv.AbsDiff(m, grayI, &diffed)
//	gocv.Threshold(diffed, &threshM, 48, 255, gocv.ThresholdBinary)
//	kernel := gocv.GetStructuringElement(gocv.MorphCross, image.Point{1, 1})
//	defer kernel.Close()
//	gocv.MorphologyExWithParams(diffed, &diffed, gocv.MorphType(gocv.MorphCross), kernel, 1, gocv.BorderIsolated)
//	gocv.Inpaint(img, diffed, &diffed, 1, 1)
//
//	//	gocv.Threshold(grayI, &threshI, 48, 255, gocv.ThresholdBinary)
//	//	gocv.BitwiseAndWithMask(img, img, &bitwiseI, threshI)
//
//	grayT := gocv.NewMat()
//	threshT := gocv.NewMat()
//	bitwiseT := gocv.NewMat()
//	defer grayT.Close()
//	defer threshT.Close()
//	defer bitwiseT.Close()
//	gocv.CvtColor(template, &grayT, gocv.ColorBGRToGray)
//	gocv.Threshold(grayT, &threshT, 48, 255, gocv.ThresholdBinary)
//	gocv.BitwiseAndWithMask(template, template, &bitwiseT, threshT)
//
//	//	kp1, des1 := sift.DetectAndCompute(bitwiseI, mask)
//	kp1, des1 := sift.DetectAndCompute(diffed, mask)
//	gocv.DrawKeyPoints(diffed, kp1, &img, color.RGBA{R: 255}, gocv.NotDrawSinglePoints)
//	w := gocv.NewWindow("test")
//	defer w.Close()
//	w.IMShow(img)
//	w.WaitKey(0)
//
//	kp2, des2 := sift.DetectAndCompute(bitwiseT, mask)
//	matcher := gocv.NewBFMatcher()
//	//	matcher := gocv.NewFlannBasedMatcher()
//	defer matcher.Close()
//
//	matches := matcher.KnnMatch(des1, des2, 2)
//
//	//	var tolerance float64
//	//	switch {
//	//	case strings.Contains(a.SearchBox.Name, "Stash"):
//	//		tolerance = 0.15
//	//	case strings.Contains(a.SearchBox.Name, "Merchant"):
//	//		tolerance = 0.2
//	//	default:
//	//		tolerance = 0.05
//	//	}
//
//	var goodMatches []gocv.DMatch
//	for _, m := range matches {
//		if len(m) > 1 {
//			if m[0].Distance < 0.1*m[1].Distance {
//				goodMatches = append(goodMatches, m[0])
//			}
//		}
//	}
//
//	//	locationMap := make(map[image.Point]int)
//
//	// Apply ratio test and group matches by location
//	//	for _, m := range matches {
//	//		if len(m) >= 2 && m[0].Distance < 0.2*m[1].Distance {
//	//			// Get the location of the match in the search image
//	//			matchLoc := kp1[m[0].TrainIdx]
//	//
//	//			// Round to nearest coordinate to group nearby matches
//	//			roundedPoint := image.Point{
//	//				X: int(matchLoc.X),
//	//				Y: int(matchLoc.Y),
//	//			}
//	//
//	//			// Group matches within a small radius
//	//			found := false
//	//			for existingPoint := range locationMap {
//	//				dx := float64(existingPoint.X - roundedPoint.X)
//	//				dy := float64(existingPoint.Y - roundedPoint.Y)
//	//				distance := dx*dx + dy*dy
//	//
//	//				if distance < 25 { // Adjust this radius based on your icon size
//	//					locationMap[existingPoint]++
//	//					found = true
//	//					break
//	//				}
//	//			}
//	//
//	//			if !found {
//	//				locationMap[roundedPoint] = 1
//	//			}
//	//		}
//	//	}
//
//	// Convert to matches array, filtering by minimum match count
//	//	var results []Match
//	var points []robotgo.Point
//
//	//	for loc, count := range locationMap {
//	//		if count >= minMatchCount {
//	//			results = append(results, Match{
//	//				Location: loc,
//	//				Score:    float64(count) / float64(len(matches)),
//	//			})
//	//			points = append(points, robotgo.Point{
//	//				X: loc.X,
//	//				Y: loc.Y,
//	//			})
//	//		}
//	//	}
//	log.Println(target)
//	log.Println(points)
//	draw := img.Clone()
//	if len(goodMatches) > 1 {
//		gocv.DrawMatches(bitwiseI, kp1, bitwiseT, kp2, goodMatches, &draw, color.RGBA{255, 0, 0, 0}, color.RGBA{0, 255, 0, 0}, nil, gocv.NotDrawSinglePoints)
//	}
//	//	w := fyne.CurrentApp().NewWindow("found images")
//	gocv.IMWrite("./internal/resources/images/FM/"+target+"FM.png", draw)
//
//	//	w.Content(canvas.NewImageFromFile())
//
//	return points
//}
//
//func (a *ImageSearch) thresholdMatching(img, template gocv.Mat) {
//	log.Println("Threshold matching...")
//	//	grayImg := gocv.NewMat()
//	//	defer grayImg.Close()
//
//	grayTemplate := gocv.NewMat()
//	defer grayTemplate.Close()
//
//	thTemplate := gocv.NewMat()
//	defer thTemplate.Close()
//
//	//	gocv.CvtColor(img, &grayImg, gocv.ColorRGBToGray)
//	gocv.CvtColor(template, &grayTemplate, gocv.ColorRGBToGray)
//
//	gocv.AdaptiveThreshold(grayTemplate, &thTemplate, 50, gocv.AdaptiveThresholdGaussian, gocv.ThresholdBinary, 11, 2)
//	window := gocv.NewWindow("Threshold")
//	defer window.Close()
//	window.IMShow(thTemplate)
//	gocv.WaitKey(0)
//}

package services

import (
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/vision"
	"image"
	_ "image/png"
	"log"
	"os"
	"slices"
	"strings"

	"github.com/go-vgo/robotgo"
)

func init() {
	registerActionRunner("imagesearch", executeImageSearch)
	registerActionRunner("ocr", executeOcr)
	registerActionRunner("findpixel", executeFindPixel)
}

func executeImageSearch(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.ImageSearch)
	log.Println("Image Search:", node.String())
	if macro != nil {
		highlightFill(macro.Name, node.GetUID(), 0)
		defer highlightClear(macro.Name, node.GetUID())
	}

	frame, searchLeftX, searchTopY, err := captureImageSearchFrame(node, macro)
	defer func() {
		if frame != nil {
			frame.Close()
		}
	}()

	results := make(map[string][]robotgo.Point)
	if err == nil && frame != nil {
		results, err = matchImageSearchFrame(frame, node, macro)
	}
	if err != nil {
		log.Printf("Image Search: %v (macro continues)", err)
		if results == nil {
			results = make(map[string][]robotgo.Point)
		}
	}

	if node.WaitTilFoundConfig.Active() && len(SortListOfPoints(results)) == 0 {
		_ = retryWhileNotFound(node.WaitTilFoundConfig, 100, func() (bool, error) {
			if frame != nil {
				frame.Close()
				frame = nil
			}
			var capErr error
			frame, searchLeftX, searchTopY, capErr = captureImageSearchFrame(node, macro)
			if capErr != nil {
				log.Printf("Image Search: %v (macro continues)", capErr)
				if results == nil {
					results = make(map[string][]robotgo.Point)
				}
				return false, nil
			}
			var matchErr error
			results, matchErr = matchImageSearchFrame(frame, node, macro)
			if matchErr != nil {
				log.Printf("Image Search: %v (macro continues)", matchErr)
				if results == nil {
					results = make(map[string][]robotgo.Point)
				}
			}
			return len(SortListOfPoints(results)) > 0, nil
		})
	}

	return runImageSearchMatches(node, macro, results, searchLeftX, searchTopY)
}

func runImageSearchMatches(node *actions.ImageSearch, macro *models.Macro, results map[string][]robotgo.Point, searchLeftX, searchTopY int) error {
	sorted := SortListOfPoints(results)
	var foundNames, notFoundNames []string
	for name, points := range results {
		if len(points) > 0 {
			foundNames = append(foundNames, name)
		} else {
			notFoundNames = append(notFoundNames, name)
		}
	}
	slices.Sort(foundNames)
	slices.Sort(notFoundNames)
	count := 0
	totalMatches := len(sorted)
	var firstPoint *robotgo.Point
	for _, np := range sorted {
		if macro != nil && totalMatches > 0 {
			highlightFill(macro.Name, node.GetUID(), float64(count)/float64(totalMatches))
		}
		point := np.Point
		count++
		point.X += searchLeftX
		point.Y += searchTopY
		if firstPoint == nil {
			firstPoint = &robotgo.Point{X: point.X, Y: point.Y}
		}

		if macro != nil {
			setCoordinateOutputs(macro, node.CoordinateOutputs, point.X, point.Y)
			if np.Name != "" {
				parts := strings.SplitN(np.Name, config.ProgramDelimiter, 2)
				if len(parts) == 2 {
					program, _ := repositories.ProgramRepo().Get(parts[0])
					if program != nil {
						itemRepo, _ := program.ItemRepo()
						if itemRepo != nil {
							item, _ := itemRepo.Get(parts[1])
							if item != nil {
								setMacroVariable(macro, "StackMax", item.StackMax)
								setMacroVariable(macro, "Cols", item.GridSize[0])
								setMacroVariable(macro, "Rows", item.GridSize[1])
								setMacroVariable(macro, "ItemName", item.Name)

								vs := IconVariantServiceInstance()
								variants, vErr := vs.GetVariants(parts[0], parts[1])
								if vErr == nil && len(variants) > 0 {
									iconPath := vs.GetVariantPath(parts[0], parts[1], variants[0])
									if f, openErr := os.Open(iconPath); openErr == nil {
										cfg, _, decErr := image.DecodeConfig(f)
										_ = f.Close()
										if decErr == nil {
											setMacroVariable(macro, "ImagePixelWidth", cfg.Width)
											setMacroVariable(macro, "ImagePixelHeight", cfg.Height)
										}
									}
								}
							}
						}
					}
				}
			}
		}

		brk, cont, err := handleLoopFlow(executeSubActions(node.SubActions, macro))
		if err != nil {
			return err
		}
		if cont {
			continue
		}
		if brk {
			break
		}
	}
	if len(sorted) == 0 && node.RunBranchOnNoFind {
		if _, _, err := handleLoopFlow(executeSubActions(node.SubActions, macro)); err != nil {
			return err
		}
	}
	if firstPoint != nil && macro != nil {
		setCoordinateOutputs(macro, node.CoordinateOutputs, firstPoint.X, firstPoint.Y)
	}
	log.Printf("Total # found: %v (found: %v; not found: %v)\n", count, foundNames, notFoundNames)
	return nil
}

func executeOcr(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Ocr)
	foundText, centerX, centerY, err := vision.OCR(node, macro)
	if err != nil {
		log.Printf("OCR: %v (macro continues)", err)
		return nil
	}
	if node.WaitTilFoundConfig.Active() && !strings.Contains(foundText, node.Target) {
		_ = retryWhileNotFound(node.WaitTilFoundConfig, 500, func() (bool, error) {
			var retryErr error
			foundText, centerX, centerY, retryErr = vision.OCR(node, macro)
			if retryErr != nil {
				log.Printf("OCR: %v (macro continues)", retryErr)
				return false, nil
			}
			return strings.Contains(foundText, node.Target), nil
		})
	}

	if macro != nil && node.OutputVariable != "" {
		setMacroVariable(macro, node.OutputVariable, foundText)
	}
	if macro != nil {
		setCoordinateOutputs(macro, node.CoordinateOutputs, centerX, centerY)
	}
	return nil
}

func executeFindPixel(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.FindPixel)
	log.Println("Find pixel:", node.String())
	leftX, topY, rightX, bottomY, err := macropkg.ResolveSearchAreaCoordsFromRef(node.SearchArea, macro, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("FindPixel: failed to resolve search area %q: %v, skipping", node.SearchArea, err)
		return nil
	}

	tr, tg, tb, colorOK := rgbFromHex(node.NormalizeHex(node.TargetColor))
	if !colorOK {
		log.Printf("FindPixel: invalid target color %q", node.TargetColor)
		return nil
	}

	var foundX, foundY int
	scanOnce := func() bool {
		captureImg, capLeftX, capTopY, _, _, capErr := macropkg.CaptureSearchArea(leftX, topY, rightX, bottomY)
		if capErr != nil || captureImg == nil {
			log.Printf("FindPixel: screen capture failed: %v", capErr)
			return false
		}
		x, y, ok := findPixelInCapture(captureImg, capLeftX, capTopY, tr, tg, tb, node.ColorTolerance)
		if ok {
			foundX, foundY = x, y
		}
		return ok
	}

	found := scanOnce()
	if !found && node.WaitTilFoundConfig.Active() {
		_ = retryWhileNotFound(node.WaitTilFoundConfig, 100, func() (bool, error) {
			found = scanOnce()
			return found, nil
		})
	}

	if found {
		log.Printf("FindPixel: found matching pixel at screen (%d, %d)", foundX, foundY)
		setCoordinateOutputs(macro, node.CoordinateOutputs, foundX, foundY)
	} else {
		log.Println("FindPixel: pixel not found")
	}
	return nil
}

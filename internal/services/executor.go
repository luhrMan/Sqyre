package services

import (
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"fmt"
	"log"
	"strings"

	"fyne.io/fyne/v2"
	"github.com/go-vgo/robotgo"
)

func Execute(a actions.ActionInterface) error {
	switch node := a.(type) {
	case *actions.Wait:
		log.Println("Wait:", node.String())
		robotgo.MilliSleep(node.Time)
		return nil
	case *actions.Move:
		log.Println("Move:", node.String())
		robotgo.Move(node.Point.X+config.XOffset, node.Point.Y+config.YOffset)
		return nil
	case *actions.Click:
		log.Println("Click:", node.String())
		robotgo.Click(actions.LeftOrRight(node.Button))
		return nil
	case *actions.Key:
		log.Println("Key:", node.String())
		if node.State {
			err := robotgo.KeyDown(node.Key)
			if err != nil {
				return err
			}
		} else {
			err := robotgo.KeyUp(node.Key)
			if err != nil {
				return err
			}
		}
		return nil

	case *actions.Loop:
		log.Println("Loop:", node.String())
		var progress, progressStep float64
		if node.Name == "root" {
			progressStep = (100.0 / float64(len(node.GetSubActions()))) / 100
			fyne.Do(func() {
				MacroActiveIndicator().Show()
				MacroActiveIndicator().Start()
			})
		}

		for i := range node.Count {
			fmt.Printf("Loop: %s iteration %d\n", node.Name, i+1)
			for j, action := range node.GetSubActions() {
				if node.Name == "root" {
					progress = progressStep * float64(j+1)
					log.Println(progress)
					fyne.Do(func() {
						MacroProgressBar().SetValue(progress)
						MacroProgressBar().Refresh()
					})
				}
				if err := Execute(action); err != nil {
					fyne.DoAndWait(func() {
						MacroActiveIndicator().Stop()
						MacroActiveIndicator().Hide()
					})
					return err
				}
			}
			if node.Name == "root" {
				fyne.Do(func() {
					MacroActiveIndicator().Stop()
					MacroActiveIndicator().Hide()
				})
			}

		}
		return nil
	case *actions.ImageSearch:
		log.Println("Image Search:", node.String())
		results, err := imageSearch(node)
		if err != nil {
			return err
		}
		sorted := SortListOfPoints(results)
		count := 0
		for _, point := range sorted {
			count++
			point.X += node.SearchArea.LeftX
			point.Y += node.SearchArea.TopY
			for _, a := range node.SubActions {
				if v, ok := a.(*actions.Move); ok {
					if v.Point.Name == "image search context" {
						v.Point.X = point.X + 25
						v.Point.Y = point.Y + 25
					}
				}
				if err := Execute(a); err != nil {
					return err
				}
			}
		}
		log.Printf("Total # found: %v\n", count)

		return nil
	case *actions.Ocr:
		foundText, err := OCR(node)
		if err != nil {
			log.Println(err)
			return err
		}
		if strings.Contains(foundText, node.Target) {
			for _, action := range node.SubActions {
				if err := Execute(action); err != nil {
					return err
				}
			}
		}
		return nil
	}
	return nil
}

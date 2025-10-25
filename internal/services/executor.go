package services

import (
	"Squire/internal/config"
	"Squire/internal/models/actions"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"github.com/go-vgo/robotgo"
)

func Execute(a actions.ActionInterface) error {
	switch node := a.(type) {
	case *actions.Wait:
		log.Printf("Waiting for %d milliseconds", node.Time)
		robotgo.MilliSleep(node.Time)
		return nil
	case *actions.Move:
		log.Printf("Moving mouse to %v", node.Point)
		robotgo.Move(node.Point.X+config.XOffset, node.Point.Y+config.YOffset)
		return nil
	case *actions.Click:
		log.Printf("%s click", actions.LeftOrRight(node.Button))
		robotgo.Click(actions.LeftOrRight(node.Button))
		return nil
	case *actions.Key:
		log.Printf("Key: %s %s", node.Key, node.State)
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
		var progress, progressStep float64
		if node.Name == "root" {
			progressStep = (100.0 / float64(len(node.GetSubActions()))) / 100
			fyne.Do(func() {
				MacroActiveIndicator().Show()
				MacroActiveIndicator().Start()
			})
		}

		for i := range node.Count {
			fmt.Printf("Loop iteration %d\n", i+1)
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
		return nil
	}
	return nil
}

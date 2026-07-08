package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/vision"
	"log"
)

func executeSemanticSearch(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.SemanticSearch)
	log.Println("Semantic Search:", node.String())
	if macro != nil {
		highlightFill(macro.Name, node.GetUID(), 0)
		defer highlightClear(macro.Name, node.GetUID())
	}

	prompt := node.Prompt
	if macro != nil {
		resolved, err := ResolveVariables(node.Prompt, macro)
		if err != nil {
			log.Printf("Semantic Search: resolve prompt: %v", err)
		} else {
			prompt = resolved
		}
	}

	var detections []struct {
		label string
		x, y  int
	}
	var originX, originY int

	scanOnce := func() int {
		dets, ox, oy, err := vision.SemanticDetect(node, macro, prompt)
		originX, originY = ox, oy
		if err != nil {
			log.Printf("Semantic Search: %v (macro continues)", err)
			return 0
		}
		detections = detections[:0]
		for _, d := range dets {
			cx, cy := d.Center()
			detections = append(detections, struct {
				label string
				x, y  int
			}{d.Label, originX + cx, originY + cy})
		}
		return len(detections)
	}

	count := scanOnce()
	if count == 0 && node.WaitTilFoundConfig.Active() {
		_ = retryWhileNotFound(node.WaitTilFoundConfig, 500, func() (bool, error) {
			count = scanOnce()
			return count > 0, nil
		})
	}

	if count == 0 {
		log.Println("Semantic Search: no matches")
		if node.RunBranchOnNoFind {
			_, _, err := handleLoopFlow(executeSubActions(node.SubActions, macro))
			return err
		}
		return nil
	}

	total := len(detections)
	for i, det := range detections {
		if macro != nil {
			highlightFill(macro.Name, node.GetUID(), float64(i)/float64(total))
			setCoordinateOutputs(macro, node.CoordinateOutputs, det.x, det.y)
			if node.OutputLabelVariable != "" {
				setMacroVariable(macro, node.OutputLabelVariable, det.label)
			}
		}
		log.Printf("Semantic Search: match %q at (%d, %d)", det.label, det.x, det.y)
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
	return nil
}

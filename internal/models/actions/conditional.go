package actions

// type ConditionalAction struct {
// 	advancedAction
// 	Condition func(any) bool
// }

// func (a *ConditionalAction) Execute(ctx any) error {
// 	if a.Condition(ctx) {
// 		fmt.Println("Condition true. Executing subactions")
// 		for _, action := range a.SubActions {
// 			if err := action.Execute(ctx); err != nil {
// 				return err
// 			}
// 		}
// 	} else {
// 		fmt.Println("Condition false. Skipping block")
// 		// for _, action := range a.FalseActions {
// 		// 	if err := action.Execute(ctx); err != nil {
// 		// 		return err
// 		// 	}
// 		// }
// 	}
// 	return nil
// }

// func (a *ConditionalAction) String() string {
// 	return fmt.Sprintf("%sConditional | %s", utils.GetEmoji("Conditional"), a.Name)
// }

// func distance(p, other robotgo.Point) float64 {
//	dx := p.X - other.X
//	dy := p.Y - other.Y
//	return math.Sqrt(float64(dx*dx + dy*dy))
//}
//
//// filterClosePoints removes points that are within minDistance of any previous point
//func filterClosePoints(points []robotgo.Point, minDistance float64) []robotgo.Point {
//	if len(points) == 0 {
//		return points
//	}
//
//	// First point is always included
//	filtered := []robotgo.Point{points[0]}
//
//	// Check each point against all previously accepted points
//	for i := 1; i < len(points); i++ {
//		tooClose := false
//		for _, accepted := range filtered {
//			dist := distance(points[i], accepted)
//			log.Printf("distance: %f", dist)
//			if dist < minDistance {
//				tooClose = true
//				break
//			}
//		}
//		if !tooClose {
//			filtered = append(filtered, points[i])
//		}
//	}
//
//	return filtered
//}

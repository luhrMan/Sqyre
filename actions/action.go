package actions

import (
	"log"

	"github.com/go-vgo/robotgo"
)

type Action interface {
	ActionType() string
	PrintParams() string
}

func performActions(actions []Action) {
	for _, action := range actions {
		switch action := action.(type) {
		case Goto:
			// Example: Implement RobotGo function to move to screen
			goTo := action.Place
			robotgo.Click(goTo)
			// Implement RobotGo function to move to screen

		case Search:
			//Search Area & Items

			// Example: Implement RobotGo function to perform search
			// searchTerm := action.Parameters["SearchTerm"].(string)
			// Implement RobotGo function to perform search

		case Click:
			// Example: Implement RobotGo function to perform click
			// coordinatesX := action.Parameters["X"].(int)
			// coordinatesY := action.Parameters["Y"].(int)
			// Implement RobotGo function to perform click

		default:
			log.Printf("Unsupported action type: %s", action.ActionType())
		}
	}
}

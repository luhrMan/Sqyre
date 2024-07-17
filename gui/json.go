package gui

import (
	"Dark-And-Darker/structs"
	"encoding/gob"
	"encoding/json"
	"fmt"
	"log"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func createSaveSettings() *fyne.Container { //fyne has a file selector / save feature i think
	macroNameEntry := widget.NewEntry()
	addSaveButton := &widget.Button{
		Text: "",
		OnTapped: func() {
			saveTreeToJsonFile(&root, macroNameEntry.Text)
		},
		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
		Icon:          theme.DocumentSaveIcon(),
		Importance:    widget.HighImportance,
	}
	return container.NewVBox(
		container.NewBorder(
			nil,
			nil,
			widget.NewLabel("Macro Name:"),
			container.NewHBox(addSaveButton),
			macroNameEntry,
		),
	)
}

func saveTreeToJsonFile(root structs.AdvancedActionInterface, filename string) error {
	// Marshal the action to JSON
	jsonData, err := json.MarshalIndent(root, "", "\t")
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}
	filepath := "./saved-macros/" + filename + ".json"
	// Write the JSON data to the file
	err = os.WriteFile(filepath, jsonData, 0644)
	if err != nil {
		return fmt.Errorf("error writing to file: %v", err)
	}
	return nil
}

func loadTreeFromJsonFile(filename string) error {
	log.Println("attempting to read file")
	log.Println(filename)
	jsonData, err := os.ReadFile("./saved-macros/" + filename)
	if err != nil {
		return fmt.Errorf("error reading file: %v", err)
	}
	log.Println(root.SubActions)
	//root = structs.LoopAction{}
	err = json.Unmarshal(jsonData, &root)

	//root, err = UnmarshalAction(jsonData, &root)
	log.Println(root.SubActions)
	if err != nil {
		return fmt.Errorf("error unmarshalling tree: %v", err)
	}
	updateTree(&tree, &root)
	return err
}

func encodeToGobFile(data *structs.LoopAction, filename string) {
	file, err := os.Create(filename)
	if err != nil {
		fmt.Println("Error creating file:", err)
		return
	}
	defer file.Close()

	// Create a new encoder and encode the data
	encoder := gob.NewEncoder(file)
	if err := encoder.Encode(data); err != nil {
		fmt.Println("Error encoding data:", err)
		return
	}

	fmt.Println("Data encoded and saved to", filename)
}

func decodeFromGobFile(filename string) structs.LoopAction {

	// Open file for reading
	file, err := os.Open(filename)
	if err != nil {
		fmt.Println("Error opening file:", err)
		// return structs.LoopAction{}
	}
	defer file.Close()

	// Create a new decoder and decode the data
	var data structs.LoopAction
	decoder := gob.NewDecoder(file)

	if err := decoder.Decode(&data); err != nil {
		fmt.Println("Error decoding data:", err)
		// return structs.LoopAction{}
	}

	return data
}

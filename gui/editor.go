package gui

// func createEditButton() *widget.Button {
// 	return widget.NewButton("Update", func() {
// 		node := findNode(root, selectedTreeItem)
// 		if selectedTreeItem == "" {
// 			log.Println("No node selected")
// 			return
// 		}
// 		og := node.String()
// 		// Type switch to handle different node types
// 		switch node := node.(type) {
// 		case *structs.WaitAction:
// 			node.Time = boundTime
// 		case *structs.MouseMoveAction:
// 			node.X, _ = strconv.Atoi(m.sections["move"].widgets["X"].(*widget.Entry).Text)
// 			node.Y, _ = strconv.Atoi(m.sections["move"].widgets["Y"].(*widget.Entry).Text)
// 		case *structs.ClickAction:
// 			node.Button = m.sections["click"].widgets["button"].(*widget.RadioGroup).Selected
// 		case *structs.KeyAction:
// 			node.Key = m.sections["key"].widgets["key"].(*widget.Select).Selected
// 			node.State = m.sections["key"].widgets["state"].(*widget.RadioGroup).Selected
// 		case *structs.LoopAction:
// 			node.Count, _ = strconv.Atoi(m.sections["loop"].widgets["count"].(*widget.Entry).Text)
// 		case *structs.ImageSearchAction:
// 			node.SearchBox = *structs.GetSearchBox(m.sections["imagesearch"].widgets["searchbox"].(*widget.Select).Selected)
// 			// node.Targets = m.sections["imagesearch"].widgets["targets"].(*widget.CheckGroup).Selected
// 			node.Targets = selectedItems()
// 		}

// 		fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

// 		// Refresh the tree to show changes
// 		tree.Refresh()
// 	})
// }

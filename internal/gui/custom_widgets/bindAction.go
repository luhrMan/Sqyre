package custom_widgets

//import (
//	"Dark-And-Darker/internal/structs"
//	"fyne.io/fyne/v2"
//	"fyne.io/fyne/v2/container"
//	"fyne.io/fyne/v2/data/binding"
//	"fyne.io/fyne/v2/widget"
//)
//
//type ActionTreeItem struct {
//	widget.BaseWidget
//
//	str      *widget.Label
//	children []*widget.Label
//
//	boundData                binding.ExternalUntyped
//	boundDataChangedListener binding.DataListener
//}
//
//func NewActionTreeItem() *ActionTreeItem {
//	item := &ActionTreeItem{
//		str:      widget.NewLabel(""),
//		children: []*widget.Label{widget.NewLabel("")},
//	}
//
//	boundDataChangedListener := binding.NewDataListener(func() {
//		value, _ := item.boundData.Get()
//		action := value.(structs.ActionInterface)
//
//		item.str.Text = action.String()
//		if s, ok := action.(structs.AdvancedActionInterface); ok {
//			for i, child := range s.GetSubActions() {
//				item.children[i].Text = child.String()
//			}
//
//			item.Refresh()
//		}
//	})
//	item.boundDataChangedListener = boundDataChangedListener
//
//	//	item.ExtendBaseWidget(item)
//
//	return item
//}
//
//func (item *ActionTreeItem) Bind(data binding.ExternalUntyped) {
//	if item.boundData != nil && item.boundDataChangedListener != nil {
//		item.boundData.RemoveListener(item.boundDataChangedListener)
//	}
//
//	item.boundData = data
//
//	if item.boundData != nil {
//		item.boundData.AddListener(item.boundDataChangedListener)
//	}
//}
//
//func (item *ActionTreeItem) CreateRenderer() fyne.WidgetRenderer {
//	c := container.NewHBox(item.str)
//	return widget.NewSimpleRenderer(c)
//}

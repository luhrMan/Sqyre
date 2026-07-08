package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestAccordionWithHeaderWidgetsOpenClose(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	acc := NewAccordionWithHeaderWidgets()
	detail0 := widget.NewLabel("detail-0")
	detail1 := widget.NewLabel("detail-1")
	acc.AppendWithHeader(widget.NewAccordionItem("A (1)", detail0), nil)
	acc.AppendWithHeader(widget.NewAccordionItem("B (1)", detail1), nil)
	acc.Resize(fyne.NewSize(300, 400))
	w.SetContent(acc)

	if acc.Items[0].Open || acc.Items[1].Open {
		t.Fatal("items should start collapsed")
	}

	acc.Open(0)
	if !acc.Items[0].Open {
		t.Fatal("item 0 should be open after Open(0)")
	}
	if acc.Items[1].Open {
		t.Fatal("item 1 should stay closed in single-open mode")
	}

	acc.Open(1)
	if acc.Items[0].Open {
		t.Fatal("item 0 should close when item 1 opens in single-open mode")
	}
	if !acc.Items[1].Open {
		t.Fatal("item 1 should be open after Open(1)")
	}

	acc.Close(1)
	if acc.Items[1].Open {
		t.Fatal("item 1 should be closed after Close(1)")
	}
}

func TestAccordionWithHeaderWidgetsOpenCloseNoOpWhenAlreadyInState(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	defer w.Close()

	acc := NewAccordionWithHeaderWidgets()
	acc.Append(widget.NewAccordionItem("A (1)", widget.NewLabel("detail")))
	acc.Resize(fyne.NewSize(200, 200))
	w.SetContent(acc)

	acc.Open(0)
	acc.Open(0) // already open
	if !acc.Items[0].Open {
		t.Fatal("item should remain open")
	}

	acc.Close(0)
	acc.Close(0) // already closed
	if acc.Items[0].Open {
		t.Fatal("item should remain closed")
	}
}

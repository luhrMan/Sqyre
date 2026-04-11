package actionrender

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// ActionIcon returns the appropriate Fyne icon resource for the given action.
func ActionIcon(a actions.ActionInterface) fyne.Resource {
	switch v := a.(type) {
	case *actions.Click:
		if v.State {
			return assets.MouseClickFilledIcon
		}
		return assets.MouseClickIcon
	case *actions.Move:
		return assets.MouseIcon
	case *actions.Key:
		if v.State {
			return theme.DownloadIcon()
		}
		return theme.UploadIcon()
	case *actions.Type:
		return theme.DocumentIcon()
	case *actions.Wait:
		return theme.HistoryIcon()
	case *actions.Loop:
		return theme.ViewRefreshIcon()
	case *actions.Ocr:
		return assets.TextSearchIcon
	case *actions.ImageSearch:
		return assets.ImageSearchIcon
	case *actions.FindPixel:
		return theme.ColorChromaticIcon()
	case *actions.SetVariable:
		return assets.VariableIcon
	case *actions.Calculate:
		return assets.CalculateIcon
	case *actions.DataList:
		return theme.StorageIcon()
	case *actions.SaveVariable:
		return theme.DocumentSaveIcon()
	case *actions.FocusWindow:
		return theme.VisibilityIcon()
	case *actions.RunMacro:
		return theme.MediaPlayIcon()
	default:
		return theme.ErrorIcon()
	}
}

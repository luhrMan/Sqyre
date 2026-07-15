package actiondisplay

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

func Icon(action actions.ActionInterface) fyne.Resource {
	switch a := action.(type) {
	case *actions.Click:
		if a.State {
			return assets.MouseClickFilledIcon
		}
		return assets.MouseClickIcon
	case *actions.Move:
		return assets.MouseIcon
	case *actions.Key:
		if a.State {
			return theme.DownloadIcon()
		}
		return theme.UploadIcon()
	case *actions.Type:
		return theme.DocumentIcon()
	case *actions.Wait:
		return theme.HistoryIcon()
	case *actions.Pause:
		return theme.MediaPauseIcon()
	case *actions.FocusWindow:
		return theme.VisibilityIcon()
	case *actions.RunMacro:
		return theme.MediaPlayIcon()
	case *actions.Conditional:
		return theme.QuestionIcon()
	case *actions.Loop:
		return theme.ViewRefreshIcon()
	case *actions.Break:
		return theme.MediaStopIcon()
	case *actions.Continue:
		return theme.MediaSkipNextIcon()
	case *actions.SetVariable:
		return assets.VariableIcon
	case *actions.SaveVariable:
		return theme.DocumentSaveIcon()
	case *actions.ForEachRow:
		return theme.ListIcon()
	case *actions.Ocr:
		return assets.TextSearchIcon
	case *actions.ImageSearch:
		return assets.ImageSearchIcon
	case *actions.FindPixel:
		return theme.ColorChromaticIcon()
	default:
		return theme.ErrorIcon()
	}
}

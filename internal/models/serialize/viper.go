package serialize

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
	"fmt"
	"os"
	"path/filepath"
	"reflect"

	"github.com/go-viper/mapstructure/v2"
	"github.com/spf13/viper"
)

type sViper struct {
	serializer
}

var Viperizer *viper.Viper

func init() {
	Viperizer = viper.New()
}

func GetViper() *viper.Viper {
	return Viperizer
}

// func (s *sViper) Encode(d any) error {
// 	// s.encodePrograms(d.(map[string]program.Program))
// 	// s.encodeMacros()
// 	log.Println("Successfully encoded:", "yaml")
// 	return nil
// }

func Decode() error {
	configPath := config.GetDbPath()

	// Ensure ~/.sqyre exists; create default config if missing
	if err := ensureConfigFile(configPath); err != nil {
		return fmt.Errorf("config setup: %w", err)
	}

	GetViper().SetConfigFile(configPath)
	GetViper().SetConfigType("yaml")
	if err := GetViper().ReadInConfig(); err != nil {
		data, readErr := os.ReadFile(configPath)
		if readErr == nil && len(data) > 0 {
			return fmt.Errorf("viper error reading config (%s): %w", configPath, YAMLErrorWithContent(data, err))
		}
		return fmt.Errorf("viper error reading in file: %v", err)
	}

	// Point YAMLConfig at the same file so repositories read/write ~/.sqyre/db.yaml
	GetYAMLConfig().SetConfigFile(configPath)
	if err := GetYAMLConfig().ReadConfig(); err != nil {
		return fmt.Errorf("yaml db read: %w", err)
	}

	return nil
}

// ensureConfigFile creates ~/.sqyre and a minimal db.yaml if the file does not exist.
func ensureConfigFile(configPath string) error {
	if _, err := os.Stat(configPath); err == nil {
		return nil
	}
	dir := filepath.Dir(configPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create config dir: %w", err)
	}
	// Write minimal config so Viper and YAMLConfig can load
	body := []byte("macros: {}\nprograms: {}\n")
	if err := os.WriteFile(configPath, body, 0644); err != nil {
		return fmt.Errorf("write default config: %w", err)
	}
	return nil
}

func MacroDecodeHookFunc() mapstructure.DecodeHookFuncType {
	return func(
		f reflect.Type,
		t reflect.Type,
		data any,
	) (any, error) {
		if t == reflect.TypeOf(actions.Loop{}) {
			rawMap, ok := data.(map[string]any)
			if !ok {
				return data, fmt.Errorf("expected map[string]any, got %T", data)
			}

			_, exists := rawMap["type"]
			if !exists {
				return data, fmt.Errorf("missing 'type' field in map")
			}

			if rawMap["type"] != "loop" {
				return data, fmt.Errorf("missing 'loop' field in map")
			}

			data, err := ViperSerializer.CreateActionFromMap(rawMap, nil)
			if err != nil {
				return data, err
			}
			return data, nil
		}
		if t == reflect.TypeOf((*actions.ActionInterface)(nil)).Elem() {
			return nil, nil
		}

		return data, nil
	}
}

type ISerializer interface {
	Encode(string, any) error
	Decode(string) (map[string]any, error)
	CreateActionFromMap(map[string]any, actions.AdvancedActionInterface) (actions.ActionInterface, error)
}

type serializer struct {
	iSerializer ISerializer
}

var (
	ViperSerializer = sViper{}
	Serializer      = serializer{}.iSerializer
)

func (s *serializer) CreateActionFromMap(rawMap map[string]any, parent actions.AdvancedActionInterface) (actions.ActionInterface, error) {
	var action actions.ActionInterface
	switch rawMap["type"] {
	case "loop":
		countVal := rawMap["count"]
		if countVal == nil {
			countVal = 1
		}
		name, err := expectString(rawMap, "name")
		if err != nil {
			return nil, fmt.Errorf("action type loop: %w", err)
		}
		action = actions.NewLoop(countVal, name, []actions.ActionInterface{})
	case "wait":
		t, err := expectInt(rawMap, "time")
		if err != nil {
			return nil, fmt.Errorf("action type wait: %w", err)
		}
		action = actions.NewWait(t)
	case "findpixel":
		name := stringFromMap(rawMap, "name")
		searchArea := actions.SearchArea{}
		if sa, ok := rawMap["searcharea"].(map[string]any); ok && len(sa) > 0 {
			searchArea = createSearchBox(sa)
		}
		targetColor := stringFromMap(rawMap, "targetcolor")
		if targetColor == "" {
			targetColor = "ffffff"
		}
		colorTolerance := intFromMap(rawMap["colortolerance"])
		if colorTolerance < 0 || colorTolerance > 100 {
			colorTolerance = 0
		}
		action = actions.NewFindPixel(name, searchArea, targetColor, colorTolerance, []actions.ActionInterface{})
		if fp, ok := action.(*actions.FindPixel); ok {
			if v, ok := rawMap["outputxvariable"].(string); ok {
				fp.OutputXVariable = v
			}
			if v, ok := rawMap["outputyvariable"].(string); ok {
				fp.OutputYVariable = v
			}
			if v, ok := rawMap["waittilfound"].(bool); ok {
				fp.WaitTilFound = v
			}
			if v := rawMap["waittilfoundseconds"]; v != nil {
				switch s := v.(type) {
				case int:
					fp.WaitTilFoundSeconds = s
				case int64:
					fp.WaitTilFoundSeconds = int(s)
				case float64:
					fp.WaitTilFoundSeconds = int(s)
				}
			}
			if v := rawMap["waittilfoundintervalms"]; v != nil {
				switch ms := v.(type) {
				case int:
					fp.WaitTilFoundIntervalMs = ms
				case int64:
					fp.WaitTilFoundIntervalMs = int(ms)
				case float64:
					fp.WaitTilFoundIntervalMs = int(ms)
				}
			}
		}
	case "click":
		button, err := expectBool(rawMap, "button")
		if err != nil {
			return nil, fmt.Errorf("action type click: %w", err)
		}
		state := false
		if v, ok := rawMap["state"].(bool); ok {
			state = v
		}
		action = actions.NewClick(button, state)
	case "move":
		pm, err := expectMap(rawMap, "point")
		if err != nil {
			return nil, fmt.Errorf("action type move: %w", err)
		}
		smooth := false
		if v, ok := rawMap["smooth"].(bool); ok {
			smooth = v
		}
		action = actions.NewMove(createPoint(pm), smooth)
	case "key":
		k, err := expectString(rawMap, "key")
		if err != nil {
			return nil, fmt.Errorf("action type key: %w", err)
		}
		st, err := expectBool(rawMap, "state")
		if err != nil {
			return nil, fmt.Errorf("action type key: %w", err)
		}
		action = actions.NewKey(k, st)
	case "type":
		text := stringFromMap(rawMap, "text")
		delayMs := 0
		if v := rawMap["delayms"]; v != nil {
			delayMs = intFromMap(v)
		}
		action = actions.NewType(text, delayMs)
	case "imagesearch":
		targets := targetsFromMap(rawMap["targets"])
		blur := 5
		if v, ok := rawMap["blur"]; ok {
			switch b := v.(type) {
			case int:
				blur = b
			case int64:
				blur = int(b)
			case float64:
				blur = int(b)
			}
		}
		name, err := expectString(rawMap, "name")
		if err != nil {
			return nil, fmt.Errorf("action type imagesearch: %w", err)
		}
		sa, err := expectMap(rawMap, "searcharea")
		if err != nil {
			return nil, fmt.Errorf("action type imagesearch: %w", err)
		}
		row, err := expectInt(rawMap, "rowsplit")
		if err != nil {
			return nil, fmt.Errorf("action type imagesearch: %w", err)
		}
		col, err := expectInt(rawMap, "colsplit")
		if err != nil {
			return nil, fmt.Errorf("action type imagesearch: %w", err)
		}
		tol, err := expectFloat64(rawMap, "tolerance")
		if err != nil {
			return nil, fmt.Errorf("action type imagesearch: %w", err)
		}
		action = actions.NewImageSearch(name, []actions.ActionInterface{}, targets, createSearchBox(sa), row, col, float32(tol), blur)
		if is, ok := action.(*actions.ImageSearch); ok {
			if v, ok := rawMap["outputxvariable"].(string); ok {
				is.OutputXVariable = v
			}
			if v, ok := rawMap["outputyvariable"].(string); ok {
				is.OutputYVariable = v
			}
			if v, ok := rawMap["waittilfound"].(bool); ok {
				is.WaitTilFound = v
			}
			if v := rawMap["waittilfoundseconds"]; v != nil {
				switch s := v.(type) {
				case int:
					is.WaitTilFoundSeconds = s
				case int64:
					is.WaitTilFoundSeconds = int(s)
				case float64:
					is.WaitTilFoundSeconds = int(s)
				}
			}
			if v := rawMap["waittilfoundintervalms"]; v != nil {
				switch ms := v.(type) {
				case int:
					is.WaitTilFoundIntervalMs = ms
				case int64:
					is.WaitTilFoundIntervalMs = int(ms)
				case float64:
					is.WaitTilFoundIntervalMs = int(ms)
				}
			}
		}
	case "ocr":
		oname, err := expectString(rawMap, "name")
		if err != nil {
			return nil, fmt.Errorf("action type ocr: %w", err)
		}
		target, err := expectString(rawMap, "target")
		if err != nil {
			return nil, fmt.Errorf("action type ocr: %w", err)
		}
		sa, err := expectMap(rawMap, "searcharea")
		if err != nil {
			return nil, fmt.Errorf("action type ocr: %w", err)
		}
		action = actions.NewOcr(oname, []actions.ActionInterface{}, target, createSearchBox(sa))
		if oc, ok := action.(*actions.Ocr); ok {
			if v, ok := rawMap["outputvariable"].(string); ok {
				oc.OutputVariable = v
			}
			if v, ok := rawMap["outputxvariable"].(string); ok {
				oc.OutputXVariable = v
			}
			if v, ok := rawMap["outputyvariable"].(string); ok {
				oc.OutputYVariable = v
			}
			if v, ok := rawMap["waittilfound"].(bool); ok {
				oc.WaitTilFound = v
			}
			if v := rawMap["waittilfoundseconds"]; v != nil {
				switch s := v.(type) {
				case int:
					oc.WaitTilFoundSeconds = s
				case int64:
					oc.WaitTilFoundSeconds = int(s)
				case float64:
					oc.WaitTilFoundSeconds = int(s)
				}
			}
			if v := rawMap["waittilfoundintervalms"]; v != nil {
				switch ms := v.(type) {
				case int:
					oc.WaitTilFoundIntervalMs = ms
				case int64:
					oc.WaitTilFoundIntervalMs = int(ms)
				case float64:
					oc.WaitTilFoundIntervalMs = int(ms)
				}
			}
		}
	case "setvariable":
		vn, err := expectString(rawMap, "variablename")
		if err != nil {
			return nil, fmt.Errorf("action type setvariable: %w", err)
		}
		action = actions.NewSetVariable(vn, rawMap["value"])
	case "calculate":
		expr, err := expectString(rawMap, "expression")
		if err != nil {
			return nil, fmt.Errorf("action type calculate: %w", err)
		}
		outv, err := expectString(rawMap, "outputvar")
		if err != nil {
			return nil, fmt.Errorf("action type calculate: %w", err)
		}
		action = actions.NewCalculate(expr, outv)
	case "datalist":
		isFile := false
		if ifVal, ok := rawMap["isfile"]; ok && ifVal != nil {
			b, ok := ifVal.(bool)
			if !ok {
				return nil, fmt.Errorf("action type datalist: field \"isfile\": expected bool, got %T", ifVal)
			}
			isFile = b
		}
		src, err := expectString(rawMap, "source")
		if err != nil {
			return nil, fmt.Errorf("action type datalist: %w", err)
		}
		outVar, err := expectString(rawMap, "outputvar")
		if err != nil {
			return nil, fmt.Errorf("action type datalist: %w", err)
		}
		action = actions.NewDataList(src, outVar, isFile)
		if dl, ok := action.(*actions.DataList); ok {
			if lv, ok := rawMap["lengthvar"].(string); ok {
				dl.LengthVar = lv
			}
			if sb, ok := rawMap["skipblanklines"].(bool); ok {
				dl.SkipBlankLines = sb
			}
		}
	case "savevariable":
		append := false
		if appendVal, ok := rawMap["append"]; ok && appendVal != nil {
			b, ok := appendVal.(bool)
			if !ok {
				return nil, fmt.Errorf("action type savevariable: field \"append\": expected bool, got %T", appendVal)
			}
			append = b
		}
		appendNewline := false
		if nlVal, ok := rawMap["appendnewline"]; ok && nlVal != nil {
			b, ok := nlVal.(bool)
			if !ok {
				return nil, fmt.Errorf("action type savevariable: field \"appendnewline\": expected bool, got %T", nlVal)
			}
			appendNewline = b
		}
		vn, err := expectString(rawMap, "variablename")
		if err != nil {
			return nil, fmt.Errorf("action type savevariable: %w", err)
		}
		dest, err := expectString(rawMap, "destination")
		if err != nil {
			return nil, fmt.Errorf("action type savevariable: %w", err)
		}
		action = actions.NewSaveVariable(vn, dest, append, appendNewline)
	// case "calibration":
	// 	name := stringFromMap(rawMap, "name")
	// 	programName := stringFromMap(rawMap, "programname")
	// 	searchArea := actions.SearchArea{}
	// 	if sa, ok := rawMap["searcharea"].(map[string]any); ok && len(sa) > 0 {
	// 		searchArea = createSearchBox(sa)
	// 	}
	// 	targets := calibrationTargetsFromMap(rawMap["targets"])
	// 	rowSplit, colSplit := 1, 1
	// 	if v := rawMap["rowsplit"]; v != nil {
	// 		rowSplit = intFromMap(v)
	// 	}
	// 	if v := rawMap["colsplit"]; v != nil {
	// 		colSplit = intFromMap(v)
	// 	}
	// 	tolerance := float32(0.95)
	// 	if v := rawMap["tolerance"]; v != nil {
	// 		switch t := v.(type) {
	// 		case float64:
	// 			tolerance = float32(t)
	// 		case float32:
	// 			tolerance = t
	// 		}
	// 	}
	// 	blur := 5
	// 	if v := rawMap["blur"]; v != nil {
	// 		blur = intFromMap(v)
	// 	}
	// action = actions.NewCalibration(name, programName, searchArea, targets, rowSplit, colSplit, tolerance, blur)
	// if cal, ok := action.(*actions.Calibration); ok {
	// 	cal.ResolutionKey = stringFromMap(rawMap, "resolutionkey")
	// }
	case "focuswindow":
		action = actions.NewFocusWindow(stringFromMap(rawMap, "windowtarget"))
	case "runmacro":
		action = actions.NewRunMacro(stringFromMap(rawMap, "macroname"))
	default:
		return nil, fmt.Errorf("unknown action type %v", rawMap["type"])
	}
	action.SetParent(parent)
	if advAction, ok := action.(actions.AdvancedActionInterface); ok {
		if subActionsRaw, ok := rawMap["subactions"].([]any); ok {
			for i, subActionRaw := range subActionsRaw {
				subMap, ok := subActionRaw.(map[string]any)
				if !ok {
					return nil, fmt.Errorf("subactions[%d]: expected mapping, got %T", i, subActionRaw)
				}
				subAction, err := s.CreateActionFromMap(subMap, advAction)
				if err != nil {
					return nil, fmt.Errorf("subactions[%d]: %w", i, err)
				}
				advAction.AddSubAction(subAction)
			}
		}
	}
	// log.Printf("Unmarshalled action %s", action)
	return action, nil
}

// targetsFromMap converts rawMap["targets"] to []string whether it is []string (from ActionToMap) or []any (from YAML).
func targetsFromMap(v any) []string {
	if v == nil {
		return nil
	}
	if ss, ok := v.([]string); ok {
		return ss
	}
	if slice, ok := v.([]any); ok {
		out := make([]string, 0, len(slice))
		for _, t := range slice {
			if s, ok := t.(string); ok {
				out = append(out, s)
			}
		}
		return out
	}
	return nil
}

func createSearchBox(rawMap map[string]any) actions.SearchArea {
	name := ""
	if n, ok := rawMap["name"].(string); ok {
		name = n
	}
	return actions.SearchArea{
		Name:    name,
		LeftX:   valueAsIntOrString(rawMap["leftx"]),
		TopY:    valueAsIntOrString(rawMap["topy"]),
		RightX:  valueAsIntOrString(rawMap["rightx"]),
		BottomY: valueAsIntOrString(rawMap["bottomy"]),
	}
}

func stringFromMap(m map[string]any, key string) string {
	if v, ok := m[key]; ok && v != nil {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}

func intFromMap(v any) int {
	if v == nil {
		return 0
	}
	switch n := v.(type) {
	case int:
		return n
	case int64:
		return int(n)
	case float64:
		return int(n)
	default:
		return 0
	}
}

// func calibrationTargetsFromMap(v any) []actions.CalibrationTarget {
// 	if v == nil {
// 		return nil
// 	}
// 	slice, ok := v.([]any)
// 	if !ok {
// 		return nil
// 	}
// 	out := make([]actions.CalibrationTarget, 0, len(slice))
// 	for _, e := range slice {
// 		m, ok := e.(map[string]any)
// 		if !ok {
// 			continue
// 		}
// 		out = append(out, actions.CalibrationTarget{
// 			OutputName: stringFromMap(m, "outputname"),
// 			OutputType: stringFromMap(m, "outputtype"),
// 			Target:     stringFromMap(m, "target"),
// 		})
// 	}
// 	return out
// }

// valueAsIntOrString converts an any to either int or string as appropriate for SearchArea fields.
func valueAsIntOrString(val any) any {
	switch v := val.(type) {
	case int:
		return v
	case float64:
		return int(v)
	case string:
		return v
	default:
		return 0
	}
}

// pointCoordFromMap returns x or y from raw map as any (int or string) for actions.Point.
func pointCoordFromMap(rawMap map[string]any, key string) any {
	v, ok := rawMap[key]
	if !ok {
		return 0
	}
	switch val := v.(type) {
	case int:
		return val
	case string:
		return val
	case float64:
		return int(val)
	default:
		return 0
	}
}

func createPoint(rawMap map[string]any) actions.Point {
	return actions.Point{
		Name: stringFromMap(rawMap, "name"),
		X:    pointCoordFromMap(rawMap, "x"),
		Y:    pointCoordFromMap(rawMap, "y"),
	}
}

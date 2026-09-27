package main

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/serialize"
)

func sampleMacro() *models.Macro {
	wait := actions.NewWait(25)
	click := actions.NewClick(actions.ClickButtonLeft, true)
	inner := actions.NewLoop(3, "inner", []actions.ActionInterface{click})
	root := actions.NewLoop(1, "root", []actions.ActionInterface{wait, inner})
	m := models.NewMacro("bench", 0, nil)
	m.Root = root
	return m
}

func macroToMap(m *models.Macro) (map[string]any, error) {
	root, err := serialize.ActionToMap(m.Root)
	if err != nil {
		return nil, err
	}
	return map[string]any{
		"name":          m.Name,
		"globaldelay":   m.GlobalDelay,
		"keyboarddelay": m.KeyboardDelay,
		"mousedelay":    m.MouseDelay,
		"hotkey":        m.Hotkey,
		"variables":     m.VariableDecls,
		"root":          root,
	}, nil
}

func benchMacroEncode(iterations uint64) sectionResult {
	m := sampleMacro()
	return timed("macro_codec_encode", iterations, "ActionToMap Wait+Loop+Click", func() error {
		_, err := macroToMap(m)
		return err
	})
}

func benchMacroDecode(iterations uint64) sectionResult {
	m := sampleMacro()
	raw, err := macroToMap(m)
	if err != nil {
		return failed("macro_codec_decode", err.Error())
	}
	return timed("macro_codec_decode", iterations, "DecodeMacroFromMap", func() error {
		_, err := serialize.DecodeMacroFromMap(raw)
		return err
	})
}

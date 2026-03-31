package appdata

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models/actions"
)

// joinProgItem builds an image-search target key: "Program~Item".
func joinProgItem(program, item string) string {
	return program + config.ProgramDelimiter + item
}

// demoSearchRect is a named screen rectangle for OCR / image search / find-pixel demos (1080p-oriented).
func demoSearchRect(name string, leftX, topY, rightX, bottomY int) actions.SearchArea {
	return actions.SearchArea{
		Name:    name,
		LeftX:   leftX,
		TopY:    topY,
		RightX:  rightX,
		BottomY: bottomY,
	}
}

func moveToFound(smooth bool) *actions.Move {
	return actions.NewMove(actions.Point{Name: "Match center", X: "${foundX}", Y: "${foundY}"}, smooth)
}

func demoMacroEldoriaOpenBags() []actions.ActionInterface {
	// Branch: critical HP tint → try quick bar potion slot.
	lowHP := actions.NewFindPixel(
		"Low HP frame tint",
		demoSearchRect("Player unit frame", 16, 380, 320, 760),
		"b71c1c",
		42,
		append([]actions.ActionInterface{actions.NewWait(40)}, keyTap("1")...),
	)
	lowHP.WaitTilFound = true
	lowHP.WaitTilFoundSeconds = 2
	lowHP.WaitTilFoundIntervalMs = 120
	lowHP.OutputXVariable = "lowHpX"
	lowHP.OutputYVariable = "lowHpY"

	// Inside bag UI: OCR sees stack/sort chrome → click to focus sort row.
	sortOCR := actions.NewOcr(
		"Bag chrome (sort / stacks)",
		append([]actions.ActionInterface{
			actions.NewWait(50),
			moveToFound(false),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(80),
		}, keyTap("r")...),
		"Sort",
		demoSearchRect("Bag interior scan", 400, 200, 1500, 880),
	)
	sortOCR.OutputVariable = "bagPanelOcr"
	sortOCR.OutputXVariable = "sortOcrX"
	sortOCR.OutputYVariable = "sortOcrY"

	// Branch: template hit on hotbar → move, open, then OCR branch above.
	hotbar := demoSearchRect("Bottom hotbar & tray", 360, 800, 1560, 1060)
	bagIcon := actions.NewImageSearch(
		"Hotbar / tray icons",
		[]actions.ActionInterface{
			actions.NewWait(55),
			moveToFound(true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(100),
			sortOCR,
		},
		[]string{
			joinProgItem("Eldoria Online", "Health potion stack"),
			joinProgItem("Eldoria Online", "Iron ore bundle"),
			joinProgItem("Eldoria Online", "Minimap expand"),
		},
		hotbar,
		1, 1, 0.15, 3,
	)
	bagIcon.WaitTilFound = true
	bagIcon.WaitTilFoundSeconds = 5
	bagIcon.OutputXVariable = "bagIconX"
	bagIcon.OutputYVariable = "bagIconY"

	// Branch: quest UI visible → prefer keyboard bags (different path than template).
	questHUD := actions.NewOcr(
		"Quest HUD visible → keyboard bags",
		append([]actions.ActionInterface{actions.NewWait(45)}, keyTap("b")...),
		"Quest",
		demoSearchRect("Quest tracker strip", 1180, 100, 1900, 440),
	)
	questHUD.OutputVariable = "questHudText"
	questHUD.OutputXVariable = "questOcrX"
	questHUD.OutputYVariable = "questOcrY"

	out := []actions.ActionInterface{
		actions.NewWait(160),
		actions.NewSetVariable("lastMacroHint", "eldoria_bags"),
		lowHP,
		actions.NewWait(80),
		bagIcon,
		actions.NewWait(70),
		questHUD,
	}
	out = append(out, actions.NewWait(50))
	out = append(out, keyTap("i")...) // character sheet alternate
	return out
}

func demoMacroHexRepeatBuild() []actions.ActionInterface {
	palette := demoSearchRect("Build palette dock", 32, 500, 540, 1000)
	placeBody := append(append([]actions.ActionInterface{}, keyTap("r")...), actions.NewWait(130))
	placeLoop := actions.NewLoop(2, "Stamp repeat builds", placeBody)

	buildHit := actions.NewImageSearch(
		"Match build palette icon",
		[]actions.ActionInterface{
			actions.NewWait(50),
			moveToFound(false),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(90),
			placeLoop,
		},
		[]string{
			joinProgItem("Hex Dominion", "Worker chip"),
			joinProgItem("Hex Dominion", "Siege workshop slot"),
			joinProgItem("Hex Dominion", "Tech tier badge"),
		},
		palette,
		1, 1, 0.14, 2,
	)
	buildHit.WaitTilFound = true
	buildHit.WaitTilFoundSeconds = 6
	buildHit.OutputXVariable = "buildIconX"
	buildHit.OutputYVariable = "buildIconY"

	// Branch: hostile marker color on minimap → alert ping.
	minimapThreat := actions.NewFindPixel(
		"Red blip on minimap",
		demoSearchRect("Minimap corner", 1580, 620, 1900, 1000),
		"d32f2f",
		48,
		append([]actions.ActionInterface{actions.NewWait(35)}, keyTap("p")...),
	)
	minimapThreat.OutputXVariable = "minimapPingX"
	minimapThreat.OutputYVariable = "minimapPingY"

	// OCR branch: mission text visible → extra confirmation wait (simulates briefing gate).
	briefing := actions.NewOcr(
		"Briefing text visible",
		[]actions.ActionInterface{
			actions.NewWait(60),
			actions.NewWait(40),
		},
		"Mission",
		demoSearchRect("Mission briefing", 520, 80, 1380, 320),
	)
	briefing.OutputVariable = "briefingSnippet"

	return []actions.ActionInterface{
		actions.NewWait(90),
		buildHit,
		actions.NewWait(100),
		minimapThreat,
		actions.NewWait(80),
		briefing,
	}
}

func demoMacroPixelSmithExport() []actions.ActionInterface {
	layersPanel := demoSearchRect("Layers panel", 0, 72, 380, 760)

	exportChord := keyChord([]string{"ctrl", "shift"}, "e")
	menuOCR := actions.NewOcr(
		"File menu shows Export",
		append(append([]actions.ActionInterface{actions.NewWait(55)}, exportChord...), actions.NewWait(220)),
		"Export",
		demoSearchRect("Top menu bar", 0, 0, 1760, 52),
	)
	menuOCR.OutputVariable = "menuOcrHit"
	menuOCR.OutputXVariable = "menuOcrX"
	menuOCR.OutputYVariable = "menuOcrY"

	layerThumb := actions.NewImageSearch(
		"Layer / preset thumbnail",
		[]actions.ActionInterface{
			actions.NewWait(50),
			moveToFound(true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(120),
			menuOCR,
		},
		[]string{
			joinProgItem("PixelSmith Studio", "Export preset WebP"),
			joinProgItem("PixelSmith Studio", "Adjustment layer"),
			joinProgItem("PixelSmith Studio", "Smart object thumb"),
		},
		layersPanel,
		1, 1, 0.13, 2,
	)
	layerThumb.WaitTilFound = true
	layerThumb.WaitTilFoundSeconds = 5
	layerThumb.OutputXVariable = "layerHitX"
	layerThumb.OutputYVariable = "layerHitY"

	histogram := actions.NewFindPixel(
		"Histogram clip warning (clip color)",
		demoSearchRect("Histogram panel", 320, 520, 620, 720),
		"ff9800",
		40,
		append([]actions.ActionInterface{actions.NewWait(40)}, keyChord([]string{"ctrl", "shift"}, "l")...),
	)
	histogram.OutputXVariable = "histX"
	histogram.OutputYVariable = "histY"

	return []actions.ActionInterface{
		actions.NewWait(120),
		layerThumb,
		actions.NewWait(90),
		histogram,
	}
}

func demoMacroNimbusInbox() []actions.ActionInterface {
	sidebar := demoSearchRect("Folder sidebar", 0, 64, 360, 940)
	inboxOCR := actions.NewOcr(
		"Inbox label in sidebar",
		append([]actions.ActionInterface{
			actions.NewWait(40),
			moveToFound(false),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(200),
		}, keyTap("/")...),
		"Inbox",
		sidebar,
	)
	inboxOCR.OutputVariable = "sidebarOcr"
	inboxOCR.OutputXVariable = "inboxOcrX"
	inboxOCR.OutputYVariable = "inboxOcrY"

	listArea := demoSearchRect("Message list", 260, 140, 1220, 960)
	unreadRow := actions.NewImageSearch(
		"Unread thread template",
		[]actions.ActionInterface{
			actions.NewWait(45),
			moveToFound(true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(150),
			actions.NewOcr(
				"Reading pane shows Subject",
				append([]actions.ActionInterface{actions.NewWait(50)}, keyTap("enter")...),
				"Subject",
				demoSearchRect("Reading pane header", 420, 120, 1500, 220),
			),
		},
		[]string{
			joinProgItem("Nimbus Mail", "Unread thread row"),
			joinProgItem("Nimbus Mail", "Calendar invite chip"),
		},
		listArea,
		1, 1, 0.14, 2,
	)
	unreadRow.WaitTilFound = true
	unreadRow.WaitTilFoundSeconds = 5
	unreadRow.OutputXVariable = "rowHitX"
	unreadRow.OutputYVariable = "rowHitY"

	promoSubs := []actions.ActionInterface{actions.NewWait(40)}
	promoSubs = append(promoSubs, keyTap("g")...)
	promoSubs = append(promoSubs, keyTap("p")...)
	promo := actions.NewOcr(
		"Promotions tab visible",
		promoSubs,
		"Promotions",
		demoSearchRect("Sidebar labels", 0, 200, 340, 520),
	)

	out := []actions.ActionInterface{
		actions.NewWait(100),
		actions.NewFocusWindow("Nimbus Mail"),
		actions.NewWait(260),
		inboxOCR,
		actions.NewWait(120),
		unreadRow,
		actions.NewWait(100),
		promo,
	}
	out = append(out, actions.NewWait(50))
	out = append(out, keyTap("g")...)
	out = append(out, keyTap("i")...)
	return out
}

func demoMacroGemDaily() []actions.ActionInterface {
	rewardStrip := demoSearchRect("Reward / streak header", 640, 120, 1280, 400)
	glow := actions.NewFindPixel(
		"Daily reward glow",
		rewardStrip,
		"ffc107",
		45,
		[]actions.ActionInterface{
			actions.NewWait(45),
			actions.NewMove(actions.Point{Name: "Glow center", X: "${foundX}", Y: "${foundY}"}, true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(90),
		},
	)
	glow.WaitTilFound = true
	glow.WaitTilFoundSeconds = 3
	glow.OutputXVariable = "glowX"
	glow.OutputYVariable = "glowY"

	hero := demoSearchRect("Home / map hero", 440, 180, 1480, 760)
	chest := actions.NewImageSearch(
		"Reward chest tile",
		[]actions.ActionInterface{
			actions.NewWait(55),
			moveToFound(true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(120),
			actions.NewOcr(
				"Claim / free label",
				append([]actions.ActionInterface{
					actions.NewWait(50),
					moveToFound(false),
					actions.NewClick(false, true),
					actions.NewClick(false, false),
				}, keyTap("space")...),
				"Claim",
				demoSearchRect("Reward modal", 520, 260, 1400, 640),
			),
		},
		[]string{
			joinProgItem("Gem Stack", "Reward video chest"),
			joinProgItem("Gem Stack", "Daily streak flame"),
			joinProgItem("Gem Stack", "Booster hammer charge"),
		},
		hero,
		1, 1, 0.15, 3,
	)
	chest.WaitTilFound = true
	chest.WaitTilFoundSeconds = 5
	chest.OutputXVariable = "chestX"
	chest.OutputYVariable = "chestY"

	lives := actions.NewFindPixel(
		"Heart / life icon pink",
		demoSearchRect("Lives row", 820, 32, 1180, 120),
		"e91e63",
		50,
		append([]actions.ActionInterface{actions.NewWait(35)}, keyTap("esc")...),
	)

	return []actions.ActionInterface{
		actions.NewWait(140),
		glow,
		actions.NewWait(80),
		chest,
		actions.NewWait(70),
		lives,
	}
}

func demoMacroStudioUndo() []actions.ActionInterface {
	layerTitle := demoSearchRect("Layers panel title", 0, 72, 300, 210)
	undoOnce := append(append([]actions.ActionInterface{}, keyChord([]string{"ctrl"}, "z")...), actions.NewWait(70))
	layerOCR := actions.NewOcr(
		"Layers panel focused",
		[]actions.ActionInterface{
			actions.NewWait(55),
			actions.NewLoop(3, "Undo burst", undoOnce),
		},
		"Layer",
		layerTitle,
	)
	layerOCR.OutputVariable = "layerTitleOcr"

	marquee := actions.NewFindPixel(
		"Selection outline (blue)",
		demoSearchRect("Canvas", 460, 100, 1700, 940),
		"2196f3",
		38,
		append([]actions.ActionInterface{actions.NewWait(40)}, keyChord([]string{"ctrl"}, "d")...),
	)
	marquee.OutputXVariable = "selectionX"
	marquee.OutputYVariable = "selectionY"

	return []actions.ActionInterface{
		actions.NewWait(100),
		layerOCR,
		actions.NewWait(90),
		marquee,
	}
}

func demoMacroMailArchive() []actions.ActionInterface {
	toolbar := demoSearchRect("Reading toolbar", 480, 68, 1420, 156)
	archiveOCR := actions.NewOcr(
		"Archive control visible",
		append([]actions.ActionInterface{
			actions.NewWait(45),
			moveToFound(false),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(120),
		}, keyTap("#")...),
		"Archive",
		toolbar,
	)
	archiveOCR.OutputVariable = "archiveOcr"
	archiveOCR.OutputXVariable = "archiveBtnX"
	archiveOCR.OutputYVariable = "archiveBtnY"

	reading := demoSearchRect("Reading pane", 380, 180, 1620, 920)
	clipHit := actions.NewImageSearch(
		"Attachment clip",
		[]actions.ActionInterface{
			actions.NewWait(40),
			moveToFound(true),
			actions.NewClick(false, true),
			actions.NewClick(false, false),
			actions.NewWait(100),
			actions.NewOcr(
				"PDF mentioned",
				append([]actions.ActionInterface{actions.NewWait(40)}, keyTap("o")...),
				"PDF",
				reading,
			),
		},
		[]string{
			joinProgItem("Nimbus Mail", "PDF attachment icon"),
			joinProgItem("Nimbus Mail", "Signature block"),
		},
		reading,
		1, 1, 0.13, 2,
	)
	clipHit.WaitTilFound = true
	clipHit.WaitTilFoundSeconds = 4
	clipHit.OutputXVariable = "clipX"
	clipHit.OutputYVariable = "clipY"

	out := []actions.ActionInterface{
		actions.NewWait(80),
		actions.NewFocusWindow("Nimbus Mail"),
		actions.NewWait(220),
		archiveOCR,
		actions.NewWait(80),
		clipHit,
		actions.NewWait(60),
	}
	out = append(out, actions.NewWait(40))
	out = append(out, keyTap("e")...)
	return out
}

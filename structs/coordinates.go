package structs

type SearchBox struct {
	AreaName string `json:"areaName"`
	LeftX    int    `json:"x1"`
	TopY     int    `json:"y1"`
	RightX   int    `json:"x2"`
	BottomY  int    `json:"y2"`
}

type Spot struct {
	SpotName string `json:"spotName"`
	X        int    `json:"x"`
	Y        int    `json:"y"`
}

type coordinate interface {
	SearchBoxMap() (string, SearchBox)
}

func SearchBoxMap() *map[string]SearchBox {
	c := make(map[string]SearchBox)
	c = map[string]SearchBox{
		"Whole Screen":       {AreaName: "Whole Screen", LeftX: 0, TopY: 0, BottomY: 1440, RightX: 2560},
		"Top Left Corner":    {AreaName: "Top Left Corner", LeftX: 0, TopY: 0, BottomY: 500, RightX: 500},
		"Top Menu Bar":       {AreaName: "Top Menu Bar", LeftX: 100, TopY: 200, BottomY: 300, RightX: 400},
		"Merchant Portraits": {AreaName: "Merchant Portraits", LeftX: 200, TopY: 300, BottomY: 400, RightX: 500},
	}
	return &c
}

func SpotMap() *map[string]Spot {
	c := make(map[string]Spot)
	c = map[string]Spot{
		"Middle":          {SpotName: "Middle", X: 2560 / 2, Y: 1440 / 2},
		"Top Left Corner": {SpotName: "Top Left Corner", X: 0, Y: 0},
		"Play Tab":        {SpotName: "Play Tab", X: 100, Y: 200},
		"Stash Tab":       {SpotName: "Stash Tab", X: 200, Y: 300},
	}
	return &c
}

func GetSearchBox(key string) SearchBox {
	sbcMap := *SearchBoxMap()
	sbc := sbcMap[key]
	return sbc
}

func GetSpot(key string) Spot {
	sscMap := *SpotMap()
	ssc := sscMap[key]
	return ssc
}

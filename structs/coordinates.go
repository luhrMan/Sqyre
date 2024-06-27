package structs

type SearchBoxCoordinates struct {
	AreaName string `json:"areaName"`
	LeftX    int    `json:"x1"`
	TopY     int    `json:"y1"`
	RightX   int    `json:"x2"`
	BottomY  int    `json:"y2"`
}

type coordinate interface {
	SearchBoxCoordinatesMap() (string, SearchBoxCoordinates)
}

func SearchBoxCoordinatesMap() *map[string]SearchBoxCoordinates {
	c := make(map[string]SearchBoxCoordinates)
	c = map[string]SearchBoxCoordinates{
		"Whole Screen":       {AreaName: "Whole Screen", LeftX: 0, TopY: 0, BottomY: 1440, RightX: 2560},
		"Top Left Corner":    {AreaName: "Top Left Corner", LeftX: 0, TopY: 0, BottomY: 500, RightX: 500},
		"Top Menu Bar":       {AreaName: "Top Menu Bar", LeftX: 100, TopY: 200, BottomY: 300, RightX: 400},
		"Merchant Portraits": {AreaName: "Merchant Portraits", LeftX: 200, TopY: 300, BottomY: 400, RightX: 500},
	}
	return &c
}

func GetSearchBoxCoordinates(key string) SearchBoxCoordinates {
	sbcMap := *SearchBoxCoordinatesMap()
	sbc := sbcMap[key]
	return sbc
}

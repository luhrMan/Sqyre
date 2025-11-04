package coordinates

import (
	"errors"
	"log"
	"slices"
	"strings"
)

type SearchArea struct {
	Name    string
	LeftX   int
	TopY    int
	RightX  int
	BottomY int
}

type Point struct {
	Name string
	X    int
	Y    int
}

func (p *Point) GetName() string {
	return p.Name
}

type Coordinates struct {
	Points      map[string]*Point
	SearchAreas map[string]*SearchArea
}

func (c *Coordinates) GetPoint(s string) (*Point, error) {
	if p, ok := c.Points[strings.ToLower(s)]; ok {
		return p, nil
	}
	return nil, errors.New("Could not get point " + s)
}
func (c *Coordinates) GetPoints() map[string]*Point {
	return c.Points
}
func (c *Coordinates) GetPointsAsStringSlice() []string {
	keys := make([]string, len(c.Points))

	i := 0
	for _, p := range c.Points {
		keys[i] = p.Name
		i++
	}
	slices.Sort(keys)
	return keys
}
func (c *Coordinates) SetPoint(p *Point) {
	c.Points[strings.ToLower(p.Name)] = p
}
func (c *Coordinates) DeletePoint(name string) {
	log.Println(strings.ToLower(name) + " point deleted")
	delete(c.Points, strings.ToLower(name))
}
func (c *Coordinates) AddPoint(p Point) (*Point, error) {
	if _, ok := c.Points[strings.ToLower(p.Name)]; ok {
		return nil, errors.New("a point with that name already exists")
	} else {
		log.Println("adding point: ", p.Name)
		c.SetPoint(&p)
		return &p, nil
	}
}

func (c *Coordinates) GetSearchArea(s string) (*SearchArea, error) {
	if sa, ok := c.SearchAreas[strings.ToLower(s)]; ok {
		return sa, nil
	}
	return nil, errors.New("Could not get search area " + s)
}

func (c *Coordinates) GetSearchAreas() map[string]*SearchArea {
	return c.SearchAreas
}
func (c *Coordinates) GetSearchAreasAsStringSlice() []string {
	keys := make([]string, len(c.SearchAreas))

	i := 0
	for k := range c.SearchAreas {
		keys[i] = k
		i++
	}

	return keys
}

func (c *Coordinates) SetSearchArea(sa *SearchArea) {
	c.SearchAreas[sa.Name] = sa
}

func (c *Coordinates) AddSearchArea(sa SearchArea) (*SearchArea, error) {
	if _, ok := c.SearchAreas[strings.ToLower(sa.Name)]; ok {
		return nil, errors.New("a search area with that name already exists")
	} else {
		log.Println("adding search area: ", sa.Name)
		c.SetSearchArea(&sa)
		return &sa, nil
	}
}

// var (
// 	path        = ResourcePath + "json/"
// 	sbMap       *map[string]SearchArea
// 	sbOnce      sync.Once
// 	spotMap     *map[string]Point
// 	spotMapOnce sync.Once
// )

// func GetSearchArea(key string) *SearchArea {
// 	m := *GetSearchAreaMap()
// 	sb := m[key]
// 	return &sb
// }

// func GetSearchAreaMapKeys(m map[string]SearchArea) *[]string {
// 	keys := make([]string, 0, len(m))
// 	for k := range m {
// 		keys = append(keys, k)
// 	}
// 	return &keys
// }

// func GetSearchAreaMap() *map[string]SearchArea {
// 	sbOnce.Do(func() {
// 		log.Println("Initializing Searchbox Map")
// 		tempArrMap := make(map[string][]SearchArea)
// 		tempMap := make(map[string]SearchArea)
// 		file, err := os.Open(path + "searchBoxes.json")
// 		if err != nil {
// 			log.Println("Error opening file:", err)
// 			panic(err)
// 		}
// 		defer file.Close()

// 		decoder := json.NewDecoder(file)
// 		if err := decoder.Decode(&tempArrMap); err != nil {
// 			log.Println("Error decoding JSON:", err)
// 			panic(err)
// 		}

// 		//		log.Println("Search Coordinates:")
// 		for _, sbArr := range tempArrMap {
// 			for _, sb := range sbArr {
// 				//				log.Printf("Area: %s, X1: %d Y1: %d X2: %d Y2: %d\n", sb.Name, sb.LeftX, sb.TopY, sb.RightX, sb.BottomY)
// 				tempMap[sb.Name] = sb
// 			}
// 		}
// 		sbMap = &tempMap
// 	})
// 	return sbMap
// }

// func GetPoint(key string) *Point {
// 	m := JsonPointMap()
// 	if s, ok := m[key]; ok {
// 		//		s := m[key]
// 		return &s
// 	}
// 	return nil
// }

// func GetPointMapKeys(m map[string]Point) *[]string {
// 	keys := make([]string, 0, len(m))
// 	for k := range m {
// 		keys = append(keys, k)
// 	}
// 	return &keys
// }

// func JsonPointMap() map[string]Point {
// 	spotMapOnce.Do(func() {
// 		tempArrMap := make(map[string][]Point)
// 		tempMap := make(map[string]Point)

// 		file, err := os.Open(path + "spots.json")
// 		if err != nil {
// 			log.Println("Error opening file:", err)
// 			panic(err)
// 		}
// 		defer file.Close()

// 		decoder := json.NewDecoder(file)
// 		if err := decoder.Decode(&tempArrMap); err != nil {
// 			log.Println("Error decoding JSON:", err)
// 			panic(err)
// 		}

// 		for _, sArr := range tempArrMap {
// 			for _, s := range sArr {
// 				//				log.Printf("Spot: %s, X: %d Y: %d\n", s.Name, s.X, s.Y)
// 				tempMap[s.Name] = s
// 			}
// 		}
// 		spotMap = &tempMap
// 	})
// 	return *spotMap
// }

// func GetPointJsonMap() map[string][]Point {
// 	spotJsonMap := make(map[string][]Point)

// 	file, err := os.Open(path + "spots.json")
// 	if err != nil {
// 		log.Println("Error opening file:", err)
// 		panic(err)
// 	}
// 	defer file.Close()

// 	decoder := json.NewDecoder(file)
// 	if err := decoder.Decode(&spotJsonMap); err != nil {
// 		log.Println("Error decoding JSON:", err)
// 		panic(err)
// 	}
// 	return spotJsonMap
// }

// func GetPointMapAsStringsMap() map[string][]string {
// 	spotStringsMap := make(map[string][]string)
// 	jsonMap := GetPointJsonMap()
// 	for str, items := range jsonMap {
// 		names := make([]string, len(items))
// 		for i, item := range items {
// 			names[i] = item.Name
// 		}
// 		//		log.Printf("strMap add names: %v", names)
// 		spotStringsMap[str] = names
// 		spotStringsMap[""] = append(spotStringsMap[""], str)
// 	}
// 	return spotStringsMap
// }

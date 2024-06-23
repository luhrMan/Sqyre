package structs

//import "encoding/json"
type ItemCategory struct{
	
}
type Item struct {
	Name		string 	`json:"name"`
	GridSize	[2]int 	`json:"gridSize"`
	StackMax 	int		`json:"stackMax"`
	Merchant 	string	`json:"merchant"`
	Category	string	`json:"category"`
}

func ItemsMap() *map[string]Item {
	i := map[string]Item{
			"Gold Purse": {
				Name:"Gold Purse",
				GridSize:[2]int{2,2},
				StackMax:0,
				Merchant:"treasurer",
				Category:"",
			},
			"Gold Purse Full": {
				Name:"Gold Purse Full",
				GridSize:[2]int{2,2},
				StackMax:50,
				Merchant:"",
				Category:"",
			},
	}
	return &i
//	data := []byte(`{"foo":"bar"}`)
//	var item
//	_ = json.Unmarshal (data,&item)
//	spew.Dump()
}
func GetItem(key string) Item {
	items := *ItemsMap()
	item := items[key]
	return item
}
package macro

const rowContentCacheMaxEntries = 200

type rowContentLRUCache struct {
	order []string
	data  map[string]cachedRowContent
}

func newRowContentLRUCache() *rowContentLRUCache {
	return &rowContentLRUCache{data: make(map[string]cachedRowContent)}
}

func (c *rowContentLRUCache) get(uid string) (cachedRowContent, bool) {
	if c == nil || uid == "" {
		return cachedRowContent{}, false
	}
	entry, ok := c.data[uid]
	if !ok {
		return cachedRowContent{}, false
	}
	c.touch(uid)
	return entry, true
}

func (c *rowContentLRUCache) put(uid string, entry cachedRowContent) {
	if c == nil || uid == "" {
		return
	}
	if c.data == nil {
		c.data = make(map[string]cachedRowContent)
	}
	if _, ok := c.data[uid]; !ok {
		c.order = append(c.order, uid)
	}
	c.data[uid] = entry
	c.touch(uid)
	for len(c.order) > rowContentCacheMaxEntries {
		evict := c.order[0]
		c.order = c.order[1:]
		delete(c.data, evict)
	}
}

func (c *rowContentLRUCache) delete(uid string) {
	if c == nil || uid == "" {
		return
	}
	delete(c.data, uid)
	for i, id := range c.order {
		if id == uid {
			c.order = append(c.order[:i], c.order[i+1:]...)
			return
		}
	}
}

func (c *rowContentLRUCache) clear() {
	if c == nil {
		return
	}
	c.order = nil
	c.data = nil
}

func (c *rowContentLRUCache) touch(uid string) {
	for i, id := range c.order {
		if id == uid {
			c.order = append(append(c.order[:i:i], c.order[i+1:]...), uid)
			return
		}
	}
}

package serialize

import "fmt"

func expectString(m map[string]any, key string) (string, error) {
	v, ok := m[key]
	if !ok || v == nil {
		return "", fmt.Errorf("missing field %q", key)
	}
	s, ok := v.(string)
	if !ok {
		return "", fmt.Errorf("field %q: expected string, got %T", key, v)
	}
	return s, nil
}

func expectBool(m map[string]any, key string) (bool, error) {
	v, ok := m[key]
	if !ok || v == nil {
		return false, fmt.Errorf("missing field %q", key)
	}
	b, ok := v.(bool)
	if !ok {
		return false, fmt.Errorf("field %q: expected bool, got %T", key, v)
	}
	return b, nil
}

func expectMap(m map[string]any, key string) (map[string]any, error) {
	v, ok := m[key]
	if !ok || v == nil {
		return nil, fmt.Errorf("missing field %q", key)
	}
	sm, ok := v.(map[string]any)
	if !ok {
		return nil, fmt.Errorf("field %q: expected mapping, got %T", key, v)
	}
	return sm, nil
}

func expectInt(m map[string]any, key string) (int, error) {
	v, ok := m[key]
	if !ok || v == nil {
		return 0, fmt.Errorf("missing field %q", key)
	}
	switch n := v.(type) {
	case int:
		return n, nil
	case int64:
		return int(n), nil
	case float64:
		return int(n), nil
	default:
		return 0, fmt.Errorf("field %q: expected number, got %T", key, v)
	}
}

func expectFloat64(m map[string]any, key string) (float64, error) {
	v, ok := m[key]
	if !ok || v == nil {
		return 0, fmt.Errorf("missing field %q", key)
	}
	switch n := v.(type) {
	case float64:
		return n, nil
	case int:
		return float64(n), nil
	case int64:
		return float64(n), nil
	default:
		return 0, fmt.Errorf("field %q: expected number, got %T", key, v)
	}
}

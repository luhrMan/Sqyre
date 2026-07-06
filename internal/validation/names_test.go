package validation

import "testing"

func TestValidateEntityName(t *testing.T) {
	if err := ValidateEntityName(""); err != ErrEmptyName {
		t.Fatalf("empty: got %v", err)
	}
	if err := ValidateEntityName("  "); err != ErrEmptyName {
		t.Fatalf("whitespace: got %v", err)
	}
	if err := ValidateEntityName("Health Potion"); err != nil {
		t.Fatalf("valid name: %v", err)
	}
}

func TestValidateVariableName(t *testing.T) {
	if err := ValidateVariableName(""); err != ErrEmptyName {
		t.Fatalf("empty: got %v", err)
	}
	for _, bad := range []string{"${x}", "a}", "{a", "a$b"} {
		if err := ValidateVariableName(bad); err == nil {
			t.Fatalf("expected invalid for %q", bad)
		}
	}
	if err := ValidateVariableName("count"); err != nil {
		t.Fatalf("valid: %v", err)
	}
}

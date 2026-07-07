package validation

import "testing"

func TestParsePositiveInt(t *testing.T) {
	if _, err := ParsePositiveInt(""); err == nil {
		t.Fatal("expected error for empty")
	}
	if _, err := ParsePositiveInt("0"); err == nil {
		t.Fatal("expected error for zero")
	}
	if _, err := ParsePositiveInt("abc"); err == nil {
		t.Fatal("expected error for non-numeric")
	}
	v, err := ParsePositiveInt("  3 ")
	if err != nil || v != 3 {
		t.Fatalf("got %d %v", v, err)
	}
}

func TestValidateItemGridFields(t *testing.T) {
	if err := ValidateItemGridFields("2", "3", "10"); err != nil {
		t.Fatal(err)
	}
	if err := ValidateItemGridFields("0", "3", "10"); err == nil {
		t.Fatal("expected cols error")
	}
	if err := ValidateItemGridFields("2", "3", "-1"); err == nil {
		t.Fatal("expected stack max error")
	}
}

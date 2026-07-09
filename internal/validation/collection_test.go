package validation

import "testing"

func TestValidateCollectionFields(t *testing.T) {
	if err := ValidateCollectionFields("area", "2", "3"); err != nil {
		t.Fatal(err)
	}
	if err := ValidateCollectionFields("", "2", "3"); err == nil {
		t.Fatal("expected error for empty search area")
	}
	if err := ValidateCollectionFields("area", "0", "3"); err == nil {
		t.Fatal("expected error for zero rows")
	}
}

package repositories

import "Sqyre/internal/testdb"

func init() {
	testdb.RegisterRepositoryReset(ResetAllForTesting)
}

package main

import "testing"

func BenchmarkMain(b *testing.B) {
	for i := 0; i < b.N; i++ {
		main()
	}
}

func BenchmarkItemCheckTree(b *testing.B) {
	for i := 0; i < b.N; i++ {
		createItemsCheckTree()
	}
}

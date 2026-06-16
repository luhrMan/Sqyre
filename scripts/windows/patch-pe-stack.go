// patch-pe-stack sets the PE optional header SizeOfStackReserve to 16MB for a
// Windows executable. Run on Linux: go run patch-pe-stack.go <path-to.exe>
// This avoids STATUS_STACK_OVERFLOW (0xC00000FD) when the default 1MB stack is too small.
package main

import (
	"debug/pe"
	"encoding/binary"
	"fmt"
	"os"
)

const (
	stackReserveMB   = 32
	stackReserveSize = stackReserveMB * 1024 * 1024 // 33554432
)

// SizeOfStackReserve offset in PE32+ optional header (IMAGE_OPTIONAL_HEADER64).
// Sum of preceding fields = 2+1+1+4+4+4+4+4+8+4+4+2+2+2+2+2+2+4+4+4+4+2+2 = 80.
const sizeOfStackReserveOffsetInOptionalHeader = 80

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <exe>\n", os.Args[0])
		os.Exit(1)
	}
	path := os.Args[1]
	peFile, err := pe.Open(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "open %s: %v\n", path, err)
		os.Exit(1)
	}
	if _, ok := peFile.OptionalHeader.(*pe.OptionalHeader64); !ok {
		peFile.Close()
		fmt.Fprintf(os.Stderr, "%s: not a PE32+ executable\n", path)
		os.Exit(1)
	}
	peFile.Close()

	// Read e_lfanew from DOS header (offset 0x3c) to get PE header offset
	out, err := os.OpenFile(path, os.O_RDWR, 0)
	if err != nil {
		fmt.Fprintf(os.Stderr, "open %s for write: %v\n", path, err)
		os.Exit(1)
	}
	defer out.Close()
	var eLfanew uint32
	if _, err := out.Seek(0x3c, 0); err != nil {
		fmt.Fprintf(os.Stderr, "seek e_lfanew: %v\n", err)
		os.Exit(1)
	}
	if err := binary.Read(out, binary.LittleEndian, &eLfanew); err != nil {
		fmt.Fprintf(os.Stderr, "read e_lfanew: %v\n", err)
		os.Exit(1)
	}
	// Optional header starts at e_lfanew + 4 (PE sig) + 20 (COFF FileHeader)
	optionalHeaderOffset := int64(eLfanew) + 4 + 20
	stackReserveFileOffset := optionalHeaderOffset + sizeOfStackReserveOffsetInOptionalHeader
	if _, err := out.Seek(stackReserveFileOffset, 0); err != nil {
		fmt.Fprintf(os.Stderr, "seek: %v\n", err)
		os.Exit(1)
	}
	var before uint64
	if err := binary.Read(out, binary.LittleEndian, &before); err != nil {
		fmt.Fprintf(os.Stderr, "read current SizeOfStackReserve: %v\n", err)
		os.Exit(1)
	}
	if _, err := out.Seek(stackReserveFileOffset, 0); err != nil {
		fmt.Fprintf(os.Stderr, "seek: %v\n", err)
		os.Exit(1)
	}
	if err := binary.Write(out, binary.LittleEndian, uint64(stackReserveSize)); err != nil {
		fmt.Fprintf(os.Stderr, "write: %v\n", err)
		os.Exit(1)
	}
	var after uint64
	if _, err := out.Seek(stackReserveFileOffset, 0); err != nil {
		fmt.Fprintf(os.Stderr, "seek verify: %v\n", err)
		os.Exit(1)
	}
	if err := binary.Read(out, binary.LittleEndian, &after); err != nil {
		fmt.Fprintf(os.Stderr, "read back: %v\n", err)
		os.Exit(1)
	}
	if after != uint64(stackReserveSize) {
		fmt.Fprintf(os.Stderr, "patch failed: wrote 0x%x but read back 0x%x (before was 0x%x)\n", stackReserveSize, after, before)
		os.Exit(1)
	}
	fmt.Printf("Patched %s: SizeOfStackReserve %d MB -> %d MB\n", path, before/(1024*1024), stackReserveMB)
}

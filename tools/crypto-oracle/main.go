package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: crypto-oracle <corpus.json>")
		os.Exit(2)
	}
	positive, negative, err := verifyCorpus(os.Args[1])
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	// Counts and schema identity are safe evidence. Cryptographic inputs and
	// outputs deliberately do not cross the command boundary.
	fmt.Printf("format-v3 oracle: %d positive and %d negative cases passed\n", positive, negative)
}

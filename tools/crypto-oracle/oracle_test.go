package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const corpusPath = "testdata/format-v3-v1.json"

func TestFormatV3Corpus(t *testing.T) {
	positive, negative, err := verifyCorpus(corpusPath)
	if err != nil {
		t.Fatal(err)
	}
	if positive != 7 || negative != 7 {
		t.Fatalf("unexpected case counts: %d positive, %d negative", positive, negative)
	}
}

func TestUnknownSchemaFailsClosed(t *testing.T) {
	mutatedCorpusFails(t, `"schema_version": 1`, `"schema_version": 2`)
}

func TestUnknownMutationFailsClosed(t *testing.T) {
	mutatedCorpusFails(
		t,
		"set kek byte 0 to 0x76 before verification",
		"set kek byte 0 to 0x75 before verification",
	)
}

func mutatedCorpusFails(t *testing.T, old, replacement string) {
	t.Helper()
	encoded, err := os.ReadFile(corpusPath)
	if err != nil {
		t.Fatal(err)
	}
	mutated := strings.Replace(string(encoded), old, replacement, 1)
	if mutated == string(encoded) {
		t.Fatal("test mutation did not change the corpus")
	}
	path := filepath.Join(t.TempDir(), "mutated.json")
	if err := os.WriteFile(path, []byte(mutated), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, _, err := verifyCorpus(path); err == nil {
		t.Fatal("mutated corpus was accepted")
	}
}

package main

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func encyclopediaStringBundle(slot uint16, value string) []byte {
	var out bytes.Buffer
	for index := uint16(0); index < 16; index++ {
		if index != slot {
			_ = binary.Write(&out, binary.LittleEndian, uint16(0))
			continue
		}
		units := []rune(value)
		_ = binary.Write(&out, binary.LittleEndian, uint16(len(units)))
		for _, unit := range units {
			_ = binary.Write(&out, binary.LittleEndian, uint16(unit))
		}
	}
	return out.Bytes()
}

func TestDecodeWindows1252EncyclopediaText(t *testing.T) {
	got, err := decodeWindows1252Text([]byte("Commander\x92s report\r\nLine two\x00\x00"))
	if err != nil {
		t.Fatal(err)
	}
	if got != "Commander’s report\nLine two" {
		t.Fatalf("decoded text = %q", got)
	}
	for _, invalid := range [][]byte{{}, {'x'}, {'x', 0, 'y', 0}, {0x81, 0}} {
		if _, err := decodeWindows1252Text(invalid); err == nil {
			t.Fatalf("expected failure for %v", invalid)
		}
	}
}

func TestDecodeEncyclopediaSourcePreservesResourceIdentity(t *testing.T) {
	text := []rawResource{{
		ID:       5952,
		Language: encyclopediaLanguageID,
		Data:     []byte("Synthetic topic.\x00"),
	}}
	bitmaps := []rawResource{{
		ID:       373,
		Language: encyclopediaLanguageID,
		Data:     encyclopediaStringBundle(15, "EDATA.042"),
	}}
	catalog, err := decodeEncyclopediaSource(text, bitmaps)
	if err != nil {
		t.Fatal(err)
	}
	if catalog.Texts[5952].Body != "Synthetic topic." {
		t.Fatalf("unexpected body: %#v", catalog.Texts[5952])
	}
	if catalog.Artwork[5967] != "EDATA.042" {
		t.Fatalf("unexpected artwork map: %#v", catalog.Artwork)
	}
	if err := validateEncyclopediaSource(catalog); err != nil {
		t.Fatal(err)
	}
}

func TestDecodeEncyclopediaSourceFailsClosed(t *testing.T) {
	validText := rawResource{ID: 1, Language: encyclopediaLanguageID, Data: []byte("x\x00")}
	validMap := rawResource{ID: 1, Language: encyclopediaLanguageID, Data: encyclopediaStringBundle(1, "EDATA.001")}
	badTexts := []rawResource{
		{ID: 0, Language: encyclopediaLanguageID, Data: []byte("x\x00")},
		{ID: 1, Language: 1036, Data: []byte("x\x00")},
		{ID: 1, Language: encyclopediaLanguageID, CodePage: 1252, Data: []byte("x\x00")},
		{ID: 1, Language: encyclopediaLanguageID, Reserved: 1, Data: []byte("x\x00")},
	}
	for _, bad := range badTexts {
		if _, err := decodeEncyclopediaSource([]rawResource{bad}, []rawResource{validMap}); err == nil {
			t.Fatalf("expected text rejection for %#v", bad)
		}
	}
	if _, err := decodeEncyclopediaSource([]rawResource{validText, validText}, []rawResource{validMap}); err == nil || !strings.Contains(err.Error(), "duplicate") {
		t.Fatalf("expected duplicate rejection, got %v", err)
	}
	badMap := rawResource{ID: 1, Language: encyclopediaLanguageID, Data: encyclopediaStringBundle(1, "../EDATA.001")}
	if _, err := decodeEncyclopediaSource([]rawResource{validText}, []rawResource{badMap}); err == nil {
		t.Fatal("expected invalid filename rejection")
	}
	for _, bad := range []rawResource{
		{ID: 0, Language: encyclopediaLanguageID, Data: encyclopediaStringBundle(1, "EDATA.001")},
		{ID: 1, Language: 1036, Data: encyclopediaStringBundle(1, "EDATA.001")},
		{ID: 1, Language: encyclopediaLanguageID, CodePage: 1252, Data: encyclopediaStringBundle(1, "EDATA.001")},
		{ID: 1, Language: encyclopediaLanguageID, Reserved: 1, Data: encyclopediaStringBundle(1, "EDATA.001")},
	} {
		if _, err := decodeEncyclopediaSource([]rawResource{validText}, []rawResource{bad}); err == nil {
			t.Fatalf("expected artwork-map rejection for %#v", bad)
		}
	}
}

func TestVerifyEncyclopediaSourceChecksCatalogAndManifest(t *testing.T) {
	catalog := encyclopediaSource{
		SchemaVersion:  encyclopediaSchemaVersion,
		LanguageID:     encyclopediaLanguageID,
		Encoding:       encyclopediaEncoding,
		SourceCodePage: 0,
		Texts:          make(map[uint16]encyclopediaText, encyclopediaExpectedTexts),
		Artwork:        make(map[uint16]string, encyclopediaExpectedMaps),
	}
	for id := uint16(1); id <= encyclopediaExpectedTexts; id++ {
		body := fmt.Sprintf("Synthetic topic %d", id)
		catalog.Texts[id] = encyclopediaText{Body: body, BodySHA256: byteSHA256([]byte(body))}
	}
	for id := uint16(1); id <= encyclopediaExpectedMaps; id++ {
		catalog.Artwork[id] = fmt.Sprintf("EDATA.%03d", id)
	}
	catalogBytes, err := json.MarshalIndent(catalog, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	catalogBytes = append(catalogBytes, '\n')
	manifest := encyclopediaSourceManifest{
		SchemaVersion: encyclopediaSchemaVersion,
		CatalogSHA256: byteSHA256(catalogBytes),
		SourceFiles: encyclopediaSourceFiles{
			TextSHA256:    strings.Repeat("a", 64),
			ArtworkSHA256: strings.Repeat("b", 64),
		},
		Counts: encyclopediaSourceCounts{Texts: encyclopediaExpectedTexts, Artwork: encyclopediaExpectedMaps},
	}
	manifestBytes, err := json.MarshalIndent(manifest, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	manifestBytes = append(manifestBytes, '\n')
	output := filepath.Join(t.TempDir(), "source.json")
	if err := os.WriteFile(output, catalogBytes, 0o600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(output+".manifest.json", manifestBytes, 0o600); err != nil {
		t.Fatal(err)
	}
	if err := verifyEncyclopediaSource(output, io.Discard); err != nil {
		t.Fatal(err)
	}

	tampered := bytes.Replace(catalogBytes, []byte("Synthetic"), []byte("Fabricated"), 1)
	if err := os.WriteFile(output, tampered, 0o600); err != nil {
		t.Fatal(err)
	}
	if err := verifyEncyclopediaSource(output, io.Discard); err == nil {
		t.Fatal("expected tampered catalog rejection")
	}
}

func TestOwnedEnglishEncyclopediaInventoryRejectsIncompleteCatalog(t *testing.T) {
	catalog, err := decodeEncyclopediaSource(
		[]rawResource{{ID: 1, Language: encyclopediaLanguageID, Data: []byte("Synthetic\x00")}},
		[]rawResource{{ID: 1, Language: encyclopediaLanguageID, Data: encyclopediaStringBundle(1, "EDATA.001")}},
	)
	if err != nil {
		t.Fatal(err)
	}
	if err := validateOwnedEnglishEncyclopediaInventory(catalog); err == nil {
		t.Fatal("expected incomplete owned-English inventory rejection")
	}
}

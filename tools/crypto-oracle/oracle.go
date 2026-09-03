package main

import (
	"crypto/hkdf"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base32"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"

	"golang.org/x/crypto/argon2"
	"golang.org/x/crypto/chacha20poly1305"
)

type corpus struct {
	SchemaVersion int            `json:"schema_version"`
	FormatVersion int            `json:"format_version"`
	ByteEncoding  string         `json:"byte_encoding"`
	Cases         []caseRecord   `json:"cases"`
	NegativeCases []negativeCase `json:"negative_cases"`
}

type caseRecord struct {
	ID        string          `json:"id"`
	Operation string          `json:"operation"`
	Input     json.RawMessage `json:"input"`
	Expected  json.RawMessage `json:"expected"`
}

type negativeCase struct {
	ID              string `json:"id"`
	BaseCase        string `json:"base_case"`
	Mutation        string `json:"mutation"`
	ExpectedFailure string `json:"expected_failure"`
}

func verifyCorpus(path string) (int, int, error) {
	encoded, err := os.ReadFile(path)
	if err != nil {
		return 0, 0, err
	}
	var document corpus
	if err := json.Unmarshal(encoded, &document); err != nil {
		return 0, 0, fmt.Errorf("decode corpus: %w", err)
	}
	if document.SchemaVersion != 1 || document.FormatVersion != 3 {
		return 0, 0, errors.New("unsupported corpus schema or format version")
	}
	if document.ByteEncoding != "lowercase_hex" {
		return 0, 0, errors.New("unsupported corpus byte encoding")
	}

	byID := make(map[string]caseRecord, len(document.Cases))
	allIDs := make(map[string]struct{}, len(document.Cases)+len(document.NegativeCases))
	for _, item := range document.Cases {
		if item.ID == "" {
			return 0, 0, errors.New("positive case has an empty id")
		}
		if _, exists := allIDs[item.ID]; exists {
			return 0, 0, fmt.Errorf("duplicate case id %q", item.ID)
		}
		allIDs[item.ID] = struct{}{}
		byID[item.ID] = item
		if err := verifyPositive(item); err != nil {
			return 0, 0, fmt.Errorf("%s: %w", item.ID, err)
		}
	}
	for _, item := range document.NegativeCases {
		if item.ID == "" {
			return 0, 0, errors.New("negative case has an empty id")
		}
		if _, exists := allIDs[item.ID]; exists {
			return 0, 0, fmt.Errorf("duplicate case id %q", item.ID)
		}
		allIDs[item.ID] = struct{}{}
		base, exists := byID[item.BaseCase]
		if !exists {
			return 0, 0, fmt.Errorf("%s: unknown base case %q", item.ID, item.BaseCase)
		}
		if err := verifyNegative(item, base); err != nil {
			return 0, 0, fmt.Errorf("%s: %w", item.ID, err)
		}
	}
	return len(document.Cases), len(document.NegativeCases), nil
}

func verifyPositive(item caseRecord) error {
	switch item.Operation {
	case "derive_subkeys_hkdf_sha256":
		return verifySubkeys(item)
	case "compute_kcv_chacha20poly1305":
		return verifyKCV(item)
	case "compute_header_hmac_sha256":
		return verifyHeaderMAC(item)
	case "derive_passphrase_kek_argon2id":
		return verifyArgon2(item)
	case "derive_recovery_kek_hkdf_sha256":
		return verifyRecoveryKEK(item)
	case "wrap_dek_chacha20poly1305":
		return verifyWrap(item)
	case "protect_page_chacha20poly1305":
		return verifyPage(item)
	default:
		return fmt.Errorf("unsupported operation %q", item.Operation)
	}
}

func verifySubkeys(item caseRecord) error {
	var input struct {
		DEK    string   `json:"dek_hex"`
		Salt   string   `json:"salt_hex"`
		Labels []string `json:"labels_utf8"`
	}
	var expected struct {
		Page   string `json:"page_key_hex"`
		Header string `json:"header_mac_key_hex"`
		Audit  string `json:"audit_key_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	if len(input.Labels) != 3 {
		return errors.New("subkey case must contain three labels")
	}
	dek, err := decodeHex(input.DEK, 32)
	if err != nil {
		return err
	}
	var salt []byte
	if input.Salt != "" {
		salt, err = decodeHex(input.Salt, -1)
		if err != nil {
			return err
		}
	}
	outputs := []string{expected.Page, expected.Header, expected.Audit}
	for index, label := range input.Labels {
		actual, err := hkdf.Key(sha256.New, dek, salt, label, 32)
		if err != nil {
			return err
		}
		if err := equalHex(actual, outputs[index]); err != nil {
			return fmt.Errorf("label %q: %w", label, err)
		}
	}
	return nil
}

func verifyKCV(item caseRecord) error {
	var input struct {
		KEK       string `json:"kek_hex"`
		Nonce     string `json:"nonce_hex"`
		Plaintext string `json:"plaintext_hex"`
		AAD       string `json:"aad_utf8"`
	}
	var expected struct {
		Ciphertext string `json:"ciphertext_and_tag_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	key, err := decodeHex(input.KEK, chacha20poly1305.KeySize)
	if err != nil {
		return err
	}
	nonce, err := decodeHex(input.Nonce, chacha20poly1305.NonceSize)
	if err != nil {
		return err
	}
	plaintext, err := decodeHex(input.Plaintext, -1)
	if err != nil {
		return err
	}
	cipher, err := chacha20poly1305.New(key)
	if err != nil {
		return err
	}
	return equalHex(cipher.Seal(nil, nonce, plaintext, []byte(input.AAD)), expected.Ciphertext)
}

type headerInput struct {
	Key          string `json:"key_hex"`
	PageSize     int    `json:"page_size"`
	HeaderRecipe struct {
		Offset  int    `json:"offset"`
		Length  int    `json:"length"`
		Formula string `json:"formula"`
	} `json:"header_plain_recipe"`
	KeyslotRecipe struct {
		Offset  int    `json:"offset"`
		Length  int    `json:"length"`
		Formula string `json:"formula"`
	} `json:"keyslot_recipe"`
	AllOtherBytes int    `json:"all_other_bytes"`
	KeyslotCount  int    `json:"keyslot_count"`
	MACInput      string `json:"mac_input"`
}

func headerPage(input headerInput) ([]byte, error) {
	if input.PageSize != 4096 || input.KeyslotCount != 1 ||
		input.HeaderRecipe.Offset != 0 || input.HeaderRecipe.Length != 104 ||
		input.HeaderRecipe.Formula != "u8(index * 3 + 1)" ||
		input.KeyslotRecipe.Offset != 136 || input.KeyslotRecipe.Length != 256 ||
		input.KeyslotRecipe.Formula != "u8(index * 5 + 2)" ||
		input.AllOtherBytes != 0 || input.MACInput != "page[0..104] || page[136..392]" {
		return nil, errors.New("unsupported header recipe")
	}
	page := make([]byte, input.PageSize)
	for index := range 104 {
		page[index] = byte(index*3 + 1)
	}
	for index := range 256 {
		page[136+index] = byte(index*5 + 2)
	}
	return page, nil
}

func verifyHeaderMAC(item caseRecord) error {
	var input headerInput
	var expected struct {
		MAC string `json:"mac_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	key, err := decodeHex(input.Key, 32)
	if err != nil {
		return err
	}
	page, err := headerPage(input)
	if err != nil {
		return err
	}
	mac := hmac.New(sha256.New, key)
	mac.Write(page[:104])
	mac.Write(page[136:392])
	return equalHex(mac.Sum(nil), expected.MAC)
}

func verifyArgon2(item caseRecord) error {
	var input struct {
		Passphrase string `json:"passphrase_utf8"`
		Salt       string `json:"salt_hex"`
		Memory     uint32 `json:"memory_kib"`
		Iterations uint32 `json:"iterations"`
		Parallel   uint8  `json:"parallelism"`
		Version    int    `json:"version"`
		Output     uint32 `json:"output_bytes"`
	}
	var expected struct {
		KEK string `json:"kek_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	if input.Version != argon2.Version {
		return fmt.Errorf("unsupported Argon2 version %d", input.Version)
	}
	salt, err := decodeHex(input.Salt, 16)
	if err != nil {
		return err
	}
	actual := argon2.IDKey([]byte(input.Passphrase), salt, input.Iterations, input.Memory, input.Parallel, input.Output)
	return equalHex(actual, expected.KEK)
}

func verifyRecoveryKEK(item caseRecord) error {
	var input struct {
		Text          string `json:"recovery_text"`
		Normalization string `json:"normalization"`
		Decoded       string `json:"decoded_hex"`
		Salt          string `json:"hkdf_salt_hex"`
		Label         string `json:"label_utf8"`
	}
	var expected struct {
		KEK string `json:"kek_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	if input.Normalization != "remove_hyphen_then_ascii_uppercase_then_base32_no_padding" || input.Salt != "" {
		return errors.New("unsupported recovery normalization or HKDF salt")
	}
	raw, err := decodeRecovery(input.Text)
	if err != nil {
		return err
	}
	if err := equalHex(raw, input.Decoded); err != nil {
		return fmt.Errorf("decoded recovery text: %w", err)
	}
	actual, err := hkdf.Key(sha256.New, raw, nil, input.Label, 32)
	if err != nil {
		return err
	}
	return equalHex(actual, expected.KEK)
}

type wrapInput struct {
	KEK   string `json:"kek_hex"`
	DEK   string `json:"dek_hex"`
	Nonce string `json:"nonce_hex"`
	AAD   struct {
		Prefix string `json:"prefix_utf8"`
		Slot   uint16 `json:"slot_index_u16_le"`
		DEKID  uint64 `json:"dek_id_u64_le"`
		Kind   byte   `json:"kind_u8"`
	} `json:"aad"`
}

func wrapAAD(input wrapInput) []byte {
	aad := append([]byte(nil), []byte(input.AAD.Prefix)...)
	aad = binary.LittleEndian.AppendUint16(aad, input.AAD.Slot)
	aad = binary.LittleEndian.AppendUint64(aad, input.AAD.DEKID)
	return append(aad, input.AAD.Kind)
}

func wrapCiphertext(input wrapInput) ([]byte, []byte, []byte, error) {
	key, err := decodeHex(input.KEK, chacha20poly1305.KeySize)
	if err != nil {
		return nil, nil, nil, err
	}
	dek, err := decodeHex(input.DEK, 32)
	if err != nil {
		return nil, nil, nil, err
	}
	nonce, err := decodeHex(input.Nonce, chacha20poly1305.NonceSize)
	if err != nil {
		return nil, nil, nil, err
	}
	cipher, err := chacha20poly1305.New(key)
	if err != nil {
		return nil, nil, nil, err
	}
	return cipher.Seal(nil, nonce, dek, wrapAAD(input)), key, nonce, nil
}

func verifyWrap(item caseRecord) error {
	var input wrapInput
	var expected struct {
		Ciphertext string `json:"ciphertext_and_tag_hex"`
		DEK        string `json:"unwrapped_dek_hex"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	ciphertext, key, nonce, err := wrapCiphertext(input)
	if err != nil {
		return err
	}
	if err := equalHex(ciphertext, expected.Ciphertext); err != nil {
		return err
	}
	cipher, err := chacha20poly1305.New(key)
	if err != nil {
		return err
	}
	opened, err := cipher.Open(nil, nonce, ciphertext, wrapAAD(input))
	if err != nil {
		return err
	}
	return equalHex(opened, expected.DEK)
}

type pageInput struct {
	PageKey     string `json:"page_key_hex"`
	Nonce       string `json:"nonce_hex"`
	PageNumber  uint64 `json:"page_number_u64_le"`
	PageVersion uint64 `json:"page_version_u64_le"`
	PageType    byte   `json:"page_type_u8"`
	Plaintext   struct {
		Length  int    `json:"length"`
		Formula string `json:"formula"`
	} `json:"plaintext_recipe"`
	FrameLayout string `json:"frame_layout"`
	AAD         string `json:"aad"`
}

func pageMaterials(input pageInput) ([]byte, []byte, []byte, []byte, error) {
	if input.Plaintext.Length != 4056 || input.Plaintext.Formula != "u8((index * 31 + 7) mod 251)" ||
		input.FrameLayout != "nonce[12] || page_version_le[8] || page_type[1] || zero[3] || ciphertext[4056] || tag[16]" ||
		input.AAD != "page_number_u64_le || page_version_u64_le || page_type_u8" {
		return nil, nil, nil, nil, errors.New("unsupported plaintext recipe")
	}
	key, err := decodeHex(input.PageKey, chacha20poly1305.KeySize)
	if err != nil {
		return nil, nil, nil, nil, err
	}
	nonce, err := decodeHex(input.Nonce, chacha20poly1305.NonceSize)
	if err != nil {
		return nil, nil, nil, nil, err
	}
	plaintext := make([]byte, input.Plaintext.Length)
	for index := range plaintext {
		plaintext[index] = byte((index*31 + 7) % 251)
	}
	aad := make([]byte, 0, 17)
	aad = binary.LittleEndian.AppendUint64(aad, input.PageNumber)
	aad = binary.LittleEndian.AppendUint64(aad, input.PageVersion)
	aad = append(aad, input.PageType)
	return key, nonce, plaintext, aad, nil
}

func protectedPage(input pageInput) ([]byte, []byte, []byte, error) {
	key, nonce, plaintext, aad, err := pageMaterials(input)
	if err != nil {
		return nil, nil, nil, err
	}
	cipher, err := chacha20poly1305.New(key)
	if err != nil {
		return nil, nil, nil, err
	}
	ciphertext := cipher.Seal(nil, nonce, plaintext, aad)
	frame := make([]byte, 24, 4096)
	copy(frame, nonce)
	binary.LittleEndian.PutUint64(frame[12:20], input.PageVersion)
	frame[20] = input.PageType
	frame = append(frame, ciphertext...)
	return frame, plaintext, aad, nil
}

func verifyPage(item caseRecord) error {
	var input pageInput
	var expected struct {
		SHA256  string `json:"frame_sha256_hex"`
		Prefix  string `json:"frame_prefix_64_hex"`
		Suffix  string `json:"frame_suffix_32_hex"`
		Version uint64 `json:"opened_page_version"`
	}
	if err := decodePair(item, &input, &expected); err != nil {
		return err
	}
	frame, plaintext, aad, err := protectedPage(input)
	if err != nil {
		return err
	}
	if len(frame) != 4096 || expected.Version != input.PageVersion {
		return errors.New("page frame length or opened version mismatch")
	}
	digest := sha256.Sum256(frame)
	if err := equalHex(digest[:], expected.SHA256); err != nil {
		return fmt.Errorf("frame digest: %w", err)
	}
	if err := equalHex(frame[:64], expected.Prefix); err != nil {
		return fmt.Errorf("frame prefix: %w", err)
	}
	if err := equalHex(frame[len(frame)-32:], expected.Suffix); err != nil {
		return fmt.Errorf("frame suffix: %w", err)
	}
	key, _ := decodeHex(input.PageKey, chacha20poly1305.KeySize)
	nonce := frame[:chacha20poly1305.NonceSize]
	cipher, _ := chacha20poly1305.New(key)
	opened, err := cipher.Open(nil, nonce, frame[24:], aad)
	if err != nil {
		return err
	}
	if !hmac.Equal(opened, plaintext) {
		return errors.New("opened page plaintext mismatch")
	}
	return nil
}

func decodePair(item caseRecord, input, expected any) error {
	if err := json.Unmarshal(item.Input, input); err != nil {
		return fmt.Errorf("decode input: %w", err)
	}
	if err := json.Unmarshal(item.Expected, expected); err != nil {
		return fmt.Errorf("decode expected output: %w", err)
	}
	return nil
}

func decodeHex(encoded string, length int) ([]byte, error) {
	if encoded != strings.ToLower(encoded) {
		return nil, errors.New("hex input is not lowercase")
	}
	decoded, err := hex.DecodeString(encoded)
	if err != nil {
		return nil, err
	}
	if length >= 0 && len(decoded) != length {
		return nil, fmt.Errorf("decoded %d bytes, expected %d", len(decoded), length)
	}
	return decoded, nil
}

func equalHex(actual []byte, expectedHex string) error {
	expected, err := decodeHex(expectedHex, len(actual))
	if err != nil {
		return err
	}
	if !hmac.Equal(actual, expected) {
		return errors.New("output differs from corpus")
	}
	return nil
}

func decodeRecovery(text string) ([]byte, error) {
	normalized := strings.ToUpper(strings.ReplaceAll(text, "-", ""))
	return base32.StdEncoding.WithPadding(base32.NoPadding).DecodeString(normalized)
}

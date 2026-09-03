package main

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"encoding/json"
	"errors"

	"golang.org/x/crypto/chacha20poly1305"
)

func verifyNegative(item negativeCase, base caseRecord) error {
	expectedMutations := map[string]string{
		"v3-kcv-wrong-key-001":             "set kek byte 0 to 0x76 before verification",
		"v3-wrap-slot-aad-001":             "set slot_index_u16_le to 4 before unwrap",
		"v3-wrap-dek-id-aad-001":           "increment dek_id_u64_le before unwrap",
		"v3-page-number-aad-001":           "increment page_number_u64_le before open",
		"v3-page-ciphertext-tamper-001":    "xor frame byte 24 with 0x01 before open",
		"v3-header-keyslot-tamper-001":     "xor page byte 141 with 0xff before verification",
		"v3-recovery-malformed-base32-001": "replace recovery_text with not-valid-base32!!!",
	}
	if expectedMutations[item.ID] != item.Mutation {
		return errors.New("unsupported mutation definition")
	}
	switch item.ID {
	case "v3-kcv-wrong-key-001":
		if item.ExpectedFailure != "wrong_key" {
			return errors.New("unexpected normalized failure")
		}
		var input struct {
			KEK   string `json:"kek_hex"`
			Nonce string `json:"nonce_hex"`
			AAD   string `json:"aad_utf8"`
		}
		var expected struct {
			Ciphertext string `json:"ciphertext_and_tag_hex"`
		}
		if err := decodePair(base, &input, &expected); err != nil {
			return err
		}
		key, _ := decodeHex(input.KEK, 32)
		key[0] = 0x76
		nonce, _ := decodeHex(input.Nonce, 12)
		ciphertext, _ := decodeHex(expected.Ciphertext, 32)
		cipher, _ := chacha20poly1305.New(key)
		if _, err := cipher.Open(nil, nonce, ciphertext, []byte(input.AAD)); err == nil {
			return errors.New("wrong KCV key was accepted")
		}
	case "v3-wrap-slot-aad-001", "v3-wrap-dek-id-aad-001":
		if item.ExpectedFailure != "wrong_key" {
			return errors.New("unexpected normalized failure")
		}
		var input wrapInput
		var expected struct {
			Ciphertext string `json:"ciphertext_and_tag_hex"`
		}
		if err := decodePair(base, &input, &expected); err != nil {
			return err
		}
		_, key, nonce, err := wrapCiphertext(input)
		if err != nil {
			return err
		}
		ciphertext, _ := decodeHex(expected.Ciphertext, 48)
		if item.ID == "v3-wrap-slot-aad-001" {
			input.AAD.Slot = 4
		} else {
			input.AAD.DEKID++
		}
		cipher, _ := chacha20poly1305.New(key)
		if _, err := cipher.Open(nil, nonce, ciphertext, wrapAAD(input)); err == nil {
			return errors.New("changed wrap AAD was accepted")
		}
	case "v3-page-number-aad-001", "v3-page-ciphertext-tamper-001":
		if item.ExpectedFailure != "auth_failed_page" {
			return errors.New("unexpected normalized failure")
		}
		var input pageInput
		if err := json.Unmarshal(base.Input, &input); err != nil {
			return err
		}
		frame, _, aad, err := protectedPage(input)
		if err != nil {
			return err
		}
		if item.ID == "v3-page-number-aad-001" {
			binary.LittleEndian.PutUint64(aad[:8], input.PageNumber+1)
		} else {
			frame[24] ^= 0x01
		}
		key, _ := decodeHex(input.PageKey, 32)
		cipher, _ := chacha20poly1305.New(key)
		if _, err := cipher.Open(nil, frame[:12], frame[24:], aad); err == nil {
			return errors.New("changed page authentication input was accepted")
		}
	case "v3-header-keyslot-tamper-001":
		if item.ExpectedFailure != "auth_failed_header" {
			return errors.New("unexpected normalized failure")
		}
		var input headerInput
		var expected struct {
			MAC string `json:"mac_hex"`
		}
		if err := decodePair(base, &input, &expected); err != nil {
			return err
		}
		key, _ := decodeHex(input.Key, 32)
		page, _ := headerPage(input)
		page[141] ^= 0xff
		mac := hmac.New(sha256.New, key)
		mac.Write(page[:104])
		mac.Write(page[136:392])
		original, _ := decodeHex(expected.MAC, 32)
		if hmac.Equal(mac.Sum(nil), original) {
			return errors.New("changed keyslot bytes retained the header MAC")
		}
	case "v3-recovery-malformed-base32-001":
		if item.ExpectedFailure != "wrong_key" {
			return errors.New("unexpected normalized failure")
		}
		if _, err := decodeRecovery("not-valid-base32!!!"); err == nil {
			return errors.New("malformed recovery text was accepted")
		}
	default:
		return errors.New("unsupported negative case")
	}
	return nil
}

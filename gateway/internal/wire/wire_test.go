package wire

import (
	"bytes"
	"errors"
	"testing"
)

func TestDecodeOpFrame_Valid(t *testing.T) {
	frame := []byte{PrefixOp, 0xDE, 0xAD, 0xBE, 0xEF}
	payload, err := DecodeOpFrame(frame)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !bytes.Equal(payload, []byte{0xDE, 0xAD, 0xBE, 0xEF}) {
		t.Fatalf("payload = %x, want DEADBEEF", payload)
	}
}

func TestDecodeOpFrame_EmptyPayloadIsValid(t *testing.T) {
	payload, err := DecodeOpFrame([]byte{PrefixOp})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(payload) != 0 {
		t.Fatalf("payload = %x, want empty", payload)
	}
}

func TestDecodeOpFrame_EmptyFrame(t *testing.T) {
	for _, frame := range [][]byte{nil, {}} {
		if _, err := DecodeOpFrame(frame); !errors.Is(err, ErrEmptyFrame) {
			t.Fatalf("err = %v, want ErrEmptyFrame", err)
		}
	}
}

func TestDecodeOpFrame_UnknownPrefix(t *testing.T) {
	_, err := DecodeOpFrame([]byte{0xFF, 0x00})
	if !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("err = %v, want ErrUnknownPrefix", err)
	}
	if msg := err.Error(); !bytes.Contains([]byte(msg), []byte("0xFF")) {
		t.Fatalf("err message = %q, want to contain 0xFF", msg)
	}
}

func TestDecodeOpFrame_SnapshotPrefixIsReserved(t *testing.T) {
	_, err := DecodeOpFrame([]byte{PrefixSnapshot, 0x00})
	if !errors.Is(err, ErrSnapshotNotSupported) {
		t.Fatalf("err = %v, want ErrSnapshotNotSupported", err)
	}
}

func TestEncodeOpFrame_RoundTrip(t *testing.T) {
	payload := []byte{0x01, 0x02, 0x03}
	frame := EncodeOpFrame(payload)
	if frame[0] != PrefixOp {
		t.Fatalf("frame[0] = %#x, want PrefixOp", frame[0])
	}
	got, err := DecodeOpFrame(frame)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !bytes.Equal(got, payload) {
		t.Fatalf("payload = %x, want %x", got, payload)
	}
}

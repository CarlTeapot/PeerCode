package wire

import (
	"errors"
	"fmt"
)

const (
	PrefixOp       byte = 0x00
	PrefixSnapshot byte = 0x01
)

var (
	ErrEmptyFrame           = errors.New("wire: empty frame")
	ErrUnknownPrefix        = errors.New("wire: unknown prefix")
	ErrSnapshotNotSupported = errors.New("wire: snapshot frames not yet supported")
)

func DecodeOpFrame(frame []byte) ([]byte, error) {
	if len(frame) == 0 {
		return nil, ErrEmptyFrame
	}
	switch frame[0] {
	case PrefixOp:
		return frame[1:], nil
	case PrefixSnapshot:
		return nil, ErrSnapshotNotSupported
	default:
		return nil, fmt.Errorf("%w: 0x%02X", ErrUnknownPrefix, frame[0])
	}
}

func EncodeOpFrame(payload []byte) []byte {
	out := make([]byte, 1+len(payload))
	out[0] = PrefixOp
	copy(out[1:], payload)
	return out
}

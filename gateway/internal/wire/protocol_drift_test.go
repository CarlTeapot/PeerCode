package wire

import "testing"

func TestPrefixConstantsMatchRustSource(t *testing.T) {
	const (
		rustOpPrefix       byte = 0x00
		rustSnapshotPrefix byte = 0x01
	)

	if PrefixOp != rustOpPrefix {
		t.Fatalf("PrefixOp = %#x, rust OP_PREFIX = %#x — protocol drift", PrefixOp, rustOpPrefix)
	}
	if PrefixSnapshot != rustSnapshotPrefix {
		t.Fatalf("PrefixSnapshot = %#x, rust SNAPSHOT_PREFIX = %#x — protocol drift", PrefixSnapshot, rustSnapshotPrefix)
	}
}

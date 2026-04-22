use super::{OP_PREFIX, SNAPSHOT_PREFIX};

#[test]
fn prefix_constants_match_go_mirror() {
    const GO_PREFIX_OP: u8 = 0x00;
    const GO_PREFIX_SNAPSHOT: u8 = 0x01;

    assert_eq!(
        OP_PREFIX, GO_PREFIX_OP,
        "OP_PREFIX drifted from gateway/internal/wire::PrefixOp"
    );
    assert_eq!(
        SNAPSHOT_PREFIX, GO_PREFIX_SNAPSHOT,
        "SNAPSHOT_PREFIX drifted from gateway/internal/wire::PrefixSnapshot"
    );
}

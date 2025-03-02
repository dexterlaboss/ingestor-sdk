use {
    solana_sdk::{
        clock::{
            Slot,
        },
    },
    log::*,
};

// Convert a slot to its bucket representation whereby lower slots are always lexically ordered
// before higher slots
pub fn slot_to_key(slot: Slot) -> String {
    format!("{slot:016x}")
}

pub fn slot_to_blocks_key(slot: Slot, use_md5: bool) -> String {
    let slot_hex = slot_to_key(slot);

    if use_md5 {
        let hash_result = md5::compute(&slot_hex);
        let truncated_hash_hex = format!("{:x}", hash_result)[..10].to_string();

        // Concatenate the truncated hash with the slot hex to form the row key
        format!("{}{}", truncated_hash_hex, slot_hex)
    } else {
        slot_hex
    }
}

pub fn slot_to_tx_by_addr_key(slot: Slot) -> String {
    slot_to_key(!slot)
}

pub fn key_to_slot(key: &str) -> Option<Slot> {
    match Slot::from_str_radix(key, 16) {
        Ok(slot) => Some(slot),
        Err(err) => {
            // bucket data is probably corrupt
            warn!("Failed to parse object key as a slot: {}: {}", key, err);
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_slot_to_key() {
        assert_eq!(slot_to_key(0), "0000000000000000");
        assert_eq!(slot_to_key(!0), "ffffffffffffffff");
    }
}
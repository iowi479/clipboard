pub fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|e| {
            let _ = &format!("time went backwards {}", e);
            unreachable!()
        })
        .as_secs()
}

#[cfg(test)]
mod tests {
    use crate::state::{BuildState, BuildStatus};
    use crate::filesystem::ContentStore;
    use crate::builder::process::BuildProcess;

    #[test]
    fn test_build_state() {
        let state = BuildState::new("test-store".to_string(), "test-hash".to_string());
        assert_eq!(state.store_id, "test-store");
        assert_eq!(state.fs_hash, "test-hash");
        
        // Check initial status
        match state.status {
            BuildStatus::NotStarted => (),
            _ => panic!("Initial status should be NotStarted"),
        }
        
        // Check build output is initially None
        assert!(state.build_output.is_none());
    }

    // Note: More complex tests would require mocking the Theater runtime
    // interfaces, which is beyond the scope of this refactoring.
}

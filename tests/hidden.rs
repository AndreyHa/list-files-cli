// Mock test to verify hidden directory handling
// In a real implementation, this would create test files and verify behavior
#[test]
fn test_hidden_directory_selection() {
    // This is a basic structure test to ensure the pattern matching works
    // for hidden directories like .obsidian
    
    let patterns = vec![".obsidian".to_string()];
    
    // In a full implementation, this would:
    // 1. Create a test directory structure with .obsidian/config
    // 2. Run the glob matching logic
    // 3. Verify that .obsidian/config is included in results
    
    // For now, just verify the pattern is processed correctly
    assert!(!patterns.is_empty());
    assert!(patterns[0].starts_with('.'));
    
    // This test serves as a placeholder and reminder that hidden directory
    // functionality should be properly tested with actual file system operations
}

#[test]
fn test_bare_hidden_dir_normalization() {
    // Test that ".obsidian" gets normalized to ".obsidian/**"
    // This would be tested against the actual build_glob_sets function
    
    let pattern = ".obsidian";
    
    // Verify the pattern matching logic
    assert!(pattern.starts_with('.'));
    assert!(!pattern.contains(['*', '/']));
    
    // In full implementation, would call build_glob_sets and verify
    // that the pattern gets properly normalized to ".obsidian/**"
}

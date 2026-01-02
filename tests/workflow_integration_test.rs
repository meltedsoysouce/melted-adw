use melted_adw::config::workflow::Workflow;

#[test]
fn test_load_example_workflow() {
    let workflow_path = concat!(env!("CARGO_MANIFEST_DIR"), "/workflows/example.toml");
    let workflow = Workflow::from_file(workflow_path).expect("Failed to load workflow");

    assert_eq!(workflow.name(), "feature-implementation");
    assert_eq!(workflow.description(), Some("新機能の実装ワークフロー"));
    assert_eq!(workflow.version(), Some("1.0.0"));
    assert_eq!(workflow.steps().len(), 3);

    let steps = workflow.steps();
    assert_eq!(steps[0].name(), "plan");
    assert_eq!(steps[1].name(), "implement");
    assert_eq!(steps[2].name(), "review");
}

#[test]
fn test_workflow_roundtrip_with_real_file() {
    let workflow_path = concat!(env!("CARGO_MANIFEST_DIR"), "/workflows/example.toml");

    // Load workflow from file
    let original = Workflow::from_file(workflow_path).expect("Failed to load workflow");

    // Convert to string
    let toml_string = original.to_string().expect("Failed to serialize");

    // Parse back from string
    let restored = Workflow::from_toml(&toml_string).expect("Failed to parse");

    // Verify they match
    assert_eq!(restored.name(), original.name());
    assert_eq!(restored.description(), original.description());
    assert_eq!(restored.version(), original.version());
    assert_eq!(restored.steps().len(), original.steps().len());
}

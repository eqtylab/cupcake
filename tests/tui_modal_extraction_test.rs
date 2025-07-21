#[cfg(test)]
mod tests {
    use cupcake::cli::tui::init::state::*;
    use cupcake::cli::tui::init::modal::centered_rect;
    use ratatui::layout::Rect;
    use tui_input::Input;

    #[test]
    fn test_centered_rect_calculation() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(60, 40, area);
        
        // Should be 60% width, 40% height
        assert_eq!(centered.width, 60);
        assert_eq!(centered.height, 20);
        
        // Should be centered
        assert_eq!(centered.x, 20); // (100 - 60) / 2
        assert_eq!(centered.y, 15); // (50 - 20) / 2
    }

    #[test]
    fn test_custom_prompt_modal_state() {
        let mut state = DiscoveryState::default();
        assert!(!state.show_custom_prompt);
        
        // Open modal
        state.show_custom_prompt = true;
        assert!(state.show_custom_prompt);
        
        // Type some text
        state.custom_prompt_input = Input::new("Test instructions".to_string());
        assert_eq!(state.custom_prompt_input.value(), "Test instructions");
        
        // Close modal
        state.show_custom_prompt = false;
        assert!(!state.show_custom_prompt);
    }

    #[test]
    fn test_extraction_task_states() {
        let task = ExtractionTask {
            file_path: "test.md".into(),
            file_name: "test.md".to_string(),
            status: TaskStatus::Queued,
            progress: 0.0,
            elapsed_ms: 0,
            rules_found: 0,
        };
        
        assert!(matches!(task.status, TaskStatus::Queued));
        assert_eq!(task.progress, 0.0);
        assert_eq!(task.rules_found, 0);
    }

    #[test]
    fn test_task_status_progression() {
        // Test all status variants
        let queued = TaskStatus::Queued;
        let in_progress = TaskStatus::InProgress;
        let complete = TaskStatus::Complete;
        let failed = TaskStatus::Failed("Error".to_string());
        
        assert!(matches!(queued, TaskStatus::Queued));
        assert!(matches!(in_progress, TaskStatus::InProgress));
        assert!(matches!(complete, TaskStatus::Complete));
        assert!(matches!(failed, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_extraction_state_initialization() {
        let mut state = ExtractionState::default();
        assert!(state.tasks.is_empty());
        assert_eq!(state.overall_progress, 0.0);
        assert!(state.custom_instructions.is_none());
        
        // Add custom instructions
        state.custom_instructions = Some("Focus on security".to_string());
        assert_eq!(state.custom_instructions.as_ref().unwrap(), "Focus on security");
        
        // Add a task
        state.tasks.push(ExtractionTask {
            file_path: "test.md".into(),
            file_name: "test.md".to_string(),
            status: TaskStatus::InProgress,
            progress: 0.5,
            elapsed_ms: 100,
            rules_found: 5,
        });
        
        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].progress, 0.5);
        assert_eq!(state.tasks[0].rules_found, 5);
    }
}
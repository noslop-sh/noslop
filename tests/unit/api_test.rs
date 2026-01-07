//! Tests for API module
//!
//! Tests error types, request/response types, and handler functions.

use noslop::storage::TaskRefs;
use serial_test::serial;
use tempfile::TempDir;

fn setup() -> TempDir {
    let temp = TempDir::new().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    temp
}

// =============================================================================
// ERROR TYPES
// =============================================================================

mod error_tests {
    use noslop::api::ApiError;

    #[test]
    fn test_error_code_not_found() {
        let err = ApiError::not_found("Task not found");
        assert_eq!(err.status_code(), 404);
        assert_eq!(err.message, "Task not found");
    }

    #[test]
    fn test_error_code_bad_request() {
        let err = ApiError::bad_request("Invalid input");
        assert_eq!(err.status_code(), 400);
        assert_eq!(err.message, "Invalid input");
    }

    #[test]
    fn test_error_code_internal() {
        let err = ApiError::internal("Something went wrong");
        assert_eq!(err.status_code(), 500);
        assert_eq!(err.message, "Something went wrong");
    }

    #[test]
    fn test_api_error_display() {
        let err = ApiError::not_found("Resource missing");
        let display = format!("{err}");
        assert!(display.contains("NOT_FOUND"));
        assert!(display.contains("Resource missing"));
    }
}

// =============================================================================
// RESPONSE TYPES
// =============================================================================

mod response_tests {
    use noslop::api::ApiResponse;

    #[test]
    fn test_api_response_success() {
        let resp: ApiResponse<String> = ApiResponse::success("hello".to_string());
        assert!(resp.success);
        assert_eq!(resp.data, Some("hello".to_string()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<()> = ApiResponse::error("NOT_FOUND", "Task not found");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, "Task not found");
    }

    #[test]
    fn test_api_response_serializes() {
        let resp: ApiResponse<String> = ApiResponse::success("test".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"test\""));
    }
}

// =============================================================================
// REQUEST TYPES
// =============================================================================

mod request_tests {
    use noslop::api::{CreateCheckRequest, CreateTaskRequest};

    #[test]
    fn test_create_task_request_deserialize() {
        let json = r#"{"title": "My Task"}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "My Task");
        assert!(req.priority.is_none());
    }

    #[test]
    fn test_create_task_request_with_priority() {
        let json = r#"{"title": "My Task", "priority": "p0"}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "My Task");
        assert_eq!(req.priority, Some("p0".to_string()));
    }

    #[test]
    fn test_create_check_request_deserialize() {
        let json = r#"{"target": "*.rs", "message": "Review code"}"#;
        let req: CreateCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.target, "*.rs");
        assert_eq!(req.message, "Review code");
        assert_eq!(req.severity, "block"); // default
    }

    #[test]
    fn test_create_check_request_with_severity() {
        let json = r#"{"target": "*.rs", "message": "Review code", "severity": "warn"}"#;
        let req: CreateCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.severity, "warn");
    }
}

// =============================================================================
// HANDLER TESTS - STATUS
// =============================================================================

mod status_handler_tests {
    use super::*;
    use noslop::api::get_status;

    #[test]
    #[serial]
    fn test_get_status_empty() {
        let _temp = setup();

        let status = get_status().unwrap();
        assert_eq!(status.tasks.total, 0);
        assert_eq!(status.tasks.pending, 0);
        assert_eq!(status.tasks.in_progress, 0);
        assert_eq!(status.tasks.done, 0);
        assert!(status.current_task.is_none());
    }

    #[test]
    #[serial]
    fn test_get_status_with_tasks() {
        let _temp = setup();

        // Create some tasks
        TaskRefs::create("Task 1", None).unwrap();
        let id2 = TaskRefs::create("Task 2", None).unwrap();
        TaskRefs::create("Task 3", None).unwrap();

        // Start one task
        TaskRefs::set_status(&id2, "in_progress").unwrap();
        TaskRefs::set_current(&id2).unwrap();

        let status = get_status().unwrap();
        assert_eq!(status.tasks.total, 3);
        assert_eq!(status.tasks.pending, 2);
        assert_eq!(status.tasks.in_progress, 1);
        assert_eq!(status.current_task, Some(id2));
    }
}

// =============================================================================
// HANDLER TESTS - TASKS
// =============================================================================

mod task_handler_tests {
    use super::*;
    use noslop::api::{
        CreateTaskRequest, complete_task, create_task, get_task, list_tasks, start_task,
    };

    #[test]
    #[serial]
    fn test_list_tasks_empty() {
        let _temp = setup();

        let tasks = list_tasks().unwrap();
        assert!(tasks.tasks.is_empty());
    }

    #[test]
    #[serial]
    fn test_list_tasks_with_data() {
        let _temp = setup();

        TaskRefs::create("Task 1", None).unwrap();
        TaskRefs::create("Task 2", Some("p0")).unwrap();

        let tasks = list_tasks().unwrap();
        assert_eq!(tasks.tasks.len(), 2);
    }

    #[test]
    #[serial]
    fn test_get_task_found() {
        let _temp = setup();

        let id = TaskRefs::create("My Task", Some("p1")).unwrap();

        let task = get_task(&id).unwrap();
        assert_eq!(task.id, id);
        assert_eq!(task.title, "My Task");
        assert_eq!(task.priority, "p1");
        assert_eq!(task.status, "pending");
    }

    #[test]
    #[serial]
    fn test_get_task_not_found() {
        let _temp = setup();

        let result = get_task("FAKE-999");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_create_task_success() {
        let _temp = setup();

        let req = CreateTaskRequest {
            title: "New Task".to_string(),
            priority: Some("p0".to_string()),
        };

        let task = create_task(&req).unwrap();
        assert!(!task.id.is_empty());
        assert_eq!(task.title, "New Task");
        assert_eq!(task.priority, "p0");
        assert_eq!(task.status, "pending");
    }

    #[test]
    #[serial]
    fn test_create_task_empty_title() {
        let _temp = setup();

        let req = CreateTaskRequest {
            title: "   ".to_string(),
            priority: None,
        };

        let result = create_task(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }

    #[test]
    #[serial]
    fn test_start_task_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();

        let result = start_task(&id).unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.status, "in_progress");

        // Verify current was set
        let current = TaskRefs::current().unwrap();
        assert_eq!(current, Some(id));
    }

    #[test]
    #[serial]
    fn test_start_task_not_found() {
        let _temp = setup();

        let result = start_task("FAKE-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_complete_task_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();

        let result = complete_task(&id).unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.status, "done");
    }

    #[test]
    #[serial]
    fn test_complete_task_clears_current() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();
        TaskRefs::set_current(&id).unwrap();
        assert_eq!(TaskRefs::current().unwrap(), Some(id.clone()));

        complete_task(&id).unwrap();

        // Current should be cleared
        assert!(TaskRefs::current().unwrap().is_none());
    }
}

// =============================================================================
// HANDLER TESTS - CHECKS
// =============================================================================

mod check_handler_tests {
    use super::*;
    use noslop::api::{CreateCheckRequest, create_check, list_checks};

    #[test]
    #[serial]
    fn test_list_checks_no_file() {
        let _temp = setup();

        let checks = list_checks().unwrap();
        assert!(checks.checks.is_empty());
    }

    #[test]
    #[serial]
    fn test_list_checks_with_data() {
        let _temp = setup();

        // Create .noslop.toml with checks
        std::fs::write(
            ".noslop.toml",
            r#"
[[check]]
target = "*.rs"
message = "Review Rust code"
severity = "block"

[[check]]
target = "*.md"
message = "Check docs"
severity = "warn"
"#,
        )
        .unwrap();

        let checks = list_checks().unwrap();
        assert_eq!(checks.checks.len(), 2);
        assert_eq!(checks.checks[0].target, "*.rs");
        assert_eq!(checks.checks[1].target, "*.md");
    }

    #[test]
    #[serial]
    fn test_create_check_success() {
        let _temp = setup();

        // Need a .noslop.toml to add to
        std::fs::write(".noslop.toml", "# noslop config\n").unwrap();

        let req = CreateCheckRequest {
            target: "src/**/*.rs".to_string(),
            message: "Review code changes".to_string(),
            severity: "block".to_string(),
        };

        let check = create_check(&req).unwrap();
        assert!(!check.id.is_empty());
        assert_eq!(check.target, "src/**/*.rs");
        assert_eq!(check.message, "Review code changes");
        assert_eq!(check.severity, "block");
    }

    #[test]
    #[serial]
    fn test_create_check_empty_target() {
        let _temp = setup();

        let req = CreateCheckRequest {
            target: "   ".to_string(),
            message: "Some message".to_string(),
            severity: "block".to_string(),
        };

        let result = create_check(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }

    #[test]
    #[serial]
    fn test_create_check_empty_message() {
        let _temp = setup();

        let req = CreateCheckRequest {
            target: "*.rs".to_string(),
            message: "   ".to_string(),
            severity: "block".to_string(),
        };

        let result = create_check(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }

    #[test]
    #[serial]
    fn test_create_check_invalid_severity() {
        let _temp = setup();

        let req = CreateCheckRequest {
            target: "*.rs".to_string(),
            message: "Check code".to_string(),
            severity: "critical".to_string(), // invalid
        };

        let result = create_check(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }
}

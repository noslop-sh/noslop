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
        assert!(req.description.is_none());
    }

    #[test]
    fn test_create_task_request_with_priority() {
        let json = r#"{"title": "My Task", "priority": "p0"}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "My Task");
        assert_eq!(req.priority, Some("p0".to_string()));
    }

    #[test]
    fn test_create_task_request_with_description() {
        let json = r#"{"title": "My Task", "description": "This is a detailed description"}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "My Task");
        assert_eq!(req.description, Some("This is a detailed description".to_string()));
    }

    #[test]
    fn test_create_check_request_deserialize() {
        let json = r#"{"scope": "*.rs", "message": "Review code"}"#;
        let req: CreateCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.scope, "*.rs");
        assert_eq!(req.message, "Review code");
        assert_eq!(req.severity, "block"); // default
    }

    #[test]
    fn test_create_check_request_with_legacy_target() {
        // Test backwards compatibility with "target" field
        let json = r#"{"target": "*.rs", "message": "Review code"}"#;
        let req: CreateCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.scope, "*.rs");
    }

    #[test]
    fn test_create_check_request_with_severity() {
        let json = r#"{"scope": "*.rs", "message": "Review code", "severity": "warn"}"#;
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
        assert_eq!(status.tasks.backlog, 0);
        assert_eq!(status.tasks.pending, 0);
        assert_eq!(status.tasks.in_progress, 0);
        assert_eq!(status.tasks.done, 0);
        assert!(status.current_task.is_none());
    }

    #[test]
    #[serial]
    fn test_get_status_with_tasks() {
        let _temp = setup();

        // Create some tasks (they start in backlog)
        TaskRefs::create("Task 1", None).unwrap();
        let id2 = TaskRefs::create("Task 2", None).unwrap();
        TaskRefs::create("Task 3", None).unwrap();

        // Start one task
        TaskRefs::set_status(&id2, "in_progress").unwrap();
        TaskRefs::set_current(&id2).unwrap();

        let status = get_status().unwrap();
        assert_eq!(status.tasks.total, 3);
        assert_eq!(status.tasks.backlog, 2); // Task 1 and 3 are in backlog
        assert_eq!(status.tasks.pending, 0);
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
        BlockerRequest, CreateTaskRequest, LinkBranchRequest, add_blocker, backlog_task,
        complete_task, create_task, delete_task, get_task, link_branch, list_tasks, remove_blocker,
        reset_task, start_task,
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
        assert_eq!(task.status, "backlog");
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
            description: None,
            priority: Some("p0".to_string()),
            concepts: vec![],
        };

        let task = create_task(&req).unwrap();
        assert!(!task.id.is_empty());
        assert_eq!(task.title, "New Task");
        assert_eq!(task.priority, "p0");
        assert_eq!(task.status, "backlog");
    }

    #[test]
    #[serial]
    fn test_create_task_with_description() {
        let _temp = setup();

        let req = CreateTaskRequest {
            title: "Task with description".to_string(),
            description: Some("This provides context for LLMs".to_string()),
            priority: None,
            concepts: vec![],
        };

        let task = create_task(&req).unwrap();
        assert!(!task.id.is_empty());
        assert_eq!(task.title, "Task with description");

        // Verify description was stored
        let detail = get_task(&task.id).unwrap();
        assert_eq!(detail.description, Some("This provides context for LLMs".to_string()));
    }

    #[test]
    #[serial]
    fn test_create_task_empty_title() {
        let _temp = setup();

        let req = CreateTaskRequest {
            title: "   ".to_string(),
            description: None,
            priority: None,
            concepts: vec![],
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

    #[test]
    #[serial]
    fn test_reset_task_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();
        TaskRefs::set_status(&id, "in_progress").unwrap();
        TaskRefs::set_current(&id).unwrap();

        let result = reset_task(&id).unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.status, "pending");

        // Current should be cleared
        assert!(TaskRefs::current().unwrap().is_none());

        // Status should be pending
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.status, "pending");
    }

    #[test]
    #[serial]
    fn test_reset_task_not_found() {
        let _temp = setup();

        let result = reset_task("FAKE-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_backlog_task_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();
        TaskRefs::set_status(&id, "pending").unwrap();
        TaskRefs::link_branch(&id, Some("feature/test")).unwrap();
        TaskRefs::set_current(&id).unwrap();

        let result = backlog_task(&id).unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.status, "backlog");

        // Branch should be unlinked
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.branch.is_none());

        // Current should be cleared
        assert!(TaskRefs::current().unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_backlog_task_not_found() {
        let _temp = setup();

        let result = backlog_task("FAKE-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_link_branch_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();

        let req = LinkBranchRequest {
            branch: Some("feature/test".to_string()),
        };
        let result = link_branch(&id, &req).unwrap();
        assert_eq!(result.id, id);

        // Verify branch was linked
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert_eq!(task.branch, Some("feature/test".to_string()));
    }

    #[test]
    #[serial]
    fn test_link_branch_unlink() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();
        TaskRefs::link_branch(&id, Some("feature/test")).unwrap();

        let req = LinkBranchRequest { branch: None };
        link_branch(&id, &req).unwrap();

        // Verify branch was unlinked
        let task = TaskRefs::get(&id).unwrap().unwrap();
        assert!(task.branch.is_none());
    }

    #[test]
    #[serial]
    fn test_link_branch_not_found() {
        let _temp = setup();

        let req = LinkBranchRequest {
            branch: Some("feature/test".to_string()),
        };
        let result = link_branch("FAKE-999", &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_delete_task_success() {
        let _temp = setup();

        let id = TaskRefs::create("Task to delete", None).unwrap();

        let result = delete_task(&id).unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.status, "deleted");

        // Verify task is gone
        assert!(TaskRefs::get(&id).unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_delete_task_clears_current() {
        let _temp = setup();

        let id = TaskRefs::create("Task", None).unwrap();
        TaskRefs::set_current(&id).unwrap();

        delete_task(&id).unwrap();

        // Current should be cleared
        assert!(TaskRefs::current().unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_delete_task_not_found() {
        let _temp = setup();

        let result = delete_task("FAKE-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_add_blocker_success() {
        let _temp = setup();

        let blocker_id = TaskRefs::create("Blocker task", None).unwrap();
        let blocked_id = TaskRefs::create("Blocked task", None).unwrap();

        let req = BlockerRequest {
            blocker_id: blocker_id.clone(),
        };
        let result = add_blocker(&blocked_id, &req).unwrap();
        assert_eq!(result.id, blocked_id);

        // Verify blocker was added
        let task = TaskRefs::get(&blocked_id).unwrap().unwrap();
        assert!(task.blocked_by.contains(&blocker_id));
    }

    #[test]
    #[serial]
    fn test_add_blocker_task_not_found() {
        let _temp = setup();

        let blocker_id = TaskRefs::create("Blocker", None).unwrap();

        let req = BlockerRequest {
            blocker_id: blocker_id.clone(),
        };
        let result = add_blocker("FAKE-999", &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_add_blocker_blocker_not_found() {
        let _temp = setup();

        let task_id = TaskRefs::create("Task", None).unwrap();

        let req = BlockerRequest {
            blocker_id: "FAKE-999".to_string(),
        };
        let result = add_blocker(&task_id, &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_remove_blocker_success() {
        let _temp = setup();

        let blocker_id = TaskRefs::create("Blocker task", None).unwrap();
        let blocked_id = TaskRefs::create("Blocked task", None).unwrap();
        TaskRefs::add_blocker(&blocked_id, &blocker_id).unwrap();

        let req = BlockerRequest {
            blocker_id: blocker_id.clone(),
        };
        let result = remove_blocker(&blocked_id, &req).unwrap();
        assert_eq!(result.id, blocked_id);

        // Verify blocker was removed
        let task = TaskRefs::get(&blocked_id).unwrap().unwrap();
        assert!(!task.blocked_by.contains(&blocker_id));
    }

    #[test]
    #[serial]
    fn test_remove_blocker_task_not_found() {
        let _temp = setup();

        let req = BlockerRequest {
            blocker_id: "TSK-1".to_string(),
        };
        let result = remove_blocker("FAKE-999", &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_complete_task_not_found() {
        let _temp = setup();

        let result = complete_task("FAKE-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
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

        // Create .noslop.toml with checks (using legacy "target" format for backwards compat test)
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
        assert_eq!(checks.checks[0].scope, "*.rs");
        assert_eq!(checks.checks[1].scope, "*.md");
    }

    #[test]
    #[serial]
    fn test_create_check_success() {
        let _temp = setup();

        // Need a .noslop.toml to add to
        std::fs::write(".noslop.toml", "# noslop config\n").unwrap();

        let req = CreateCheckRequest {
            scope: "src/**/*.rs".to_string(),
            message: "Review code changes".to_string(),
            severity: "block".to_string(),
        };

        let check = create_check(&req).unwrap();
        assert!(!check.id.is_empty());
        assert_eq!(check.scope, "src/**/*.rs");
        assert_eq!(check.message, "Review code changes");
        assert_eq!(check.severity, "block");
    }

    #[test]
    #[serial]
    fn test_create_check_empty_scope() {
        let _temp = setup();

        let req = CreateCheckRequest {
            scope: "   ".to_string(),
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
            scope: "*.rs".to_string(),
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
            scope: "*.rs".to_string(),
            message: "Check code".to_string(),
            severity: "critical".to_string(), // invalid
        };

        let result = create_check(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }
}

// =============================================================================
// HANDLER TESTS - CONCEPTS
// =============================================================================

mod concept_handler_tests {
    use super::*;
    use noslop::api::{
        CreateConceptRequest, SelectConceptRequest, UpdateConceptRequest, create_concept,
        delete_concept, list_concepts, select_concept, update_concept,
    };

    #[test]
    #[serial]
    fn test_list_concepts_empty() {
        let _temp = setup();

        let result = list_concepts().unwrap();
        assert!(result.concepts.is_empty());
        assert!(result.current_concept.is_none());
    }

    #[test]
    #[serial]
    fn test_create_concept_success() {
        let _temp = setup();

        let req = CreateConceptRequest {
            name: "My Concept".to_string(),
            description: None,
        };
        let result = create_concept(&req).unwrap();
        assert_eq!(result.id, "CON-1");
        assert_eq!(result.name, "My Concept");

        // Verify it appears in list
        let concepts = list_concepts().unwrap();
        assert_eq!(concepts.concepts.len(), 1);
        assert_eq!(concepts.concepts[0].name, "My Concept");
        assert!(concepts.concepts[0].description.is_none());
    }

    #[test]
    #[serial]
    fn test_create_concept_with_description() {
        let _temp = setup();

        let req = CreateConceptRequest {
            name: "Auth Concept".to_string(),
            description: Some("JWT-based authentication using RS256".to_string()),
        };
        let result = create_concept(&req).unwrap();
        assert_eq!(result.id, "CON-1");
        assert_eq!(result.name, "Auth Concept");

        // Verify description is stored
        let concepts = list_concepts().unwrap();
        assert_eq!(concepts.concepts.len(), 1);
        assert_eq!(
            concepts.concepts[0].description,
            Some("JWT-based authentication using RS256".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_update_concept_description() {
        let _temp = setup();

        // Create a concept
        let req = CreateConceptRequest {
            name: "Update Me".to_string(),
            description: None,
        };
        let created = create_concept(&req).unwrap();

        // Update its description
        let update_req = UpdateConceptRequest {
            description: Some("New description".to_string()),
        };
        let result = update_concept(&created.id, &update_req).unwrap();
        assert_eq!(result.description, Some("New description".to_string()));

        // Clear the description
        let clear_req = UpdateConceptRequest { description: None };
        let result = update_concept(&created.id, &clear_req).unwrap();
        assert!(result.description.is_none());
    }

    #[test]
    #[serial]
    fn test_update_concept_not_found() {
        let _temp = setup();

        let req = UpdateConceptRequest {
            description: Some("test".to_string()),
        };
        let result = update_concept("CON-999", &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_create_concept_empty_name() {
        let _temp = setup();

        let req = CreateConceptRequest {
            name: "   ".to_string(),
            description: None,
        };
        let result = create_concept(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 400);
    }

    #[test]
    #[serial]
    fn test_delete_concept_success() {
        let _temp = setup();

        // Create a concept first
        let req = CreateConceptRequest {
            name: "To Delete".to_string(),
            description: None,
        };
        let created = create_concept(&req).unwrap();

        // Delete it
        let result = delete_concept(&created.id);
        assert!(result.is_ok());

        // Verify it's gone
        let concepts = list_concepts().unwrap();
        assert!(concepts.concepts.is_empty());
    }

    #[test]
    #[serial]
    fn test_delete_concept_not_found() {
        let _temp = setup();

        let result = delete_concept("CON-999");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_select_concept_success() {
        let _temp = setup();

        // Create a concept
        let req = CreateConceptRequest {
            name: "Select Me".to_string(),
            description: None,
        };
        let created = create_concept(&req).unwrap();

        // Select it
        let select_req = SelectConceptRequest {
            id: Some(created.id.clone()),
        };
        let result = select_concept(&select_req).unwrap();
        assert_eq!(result.current_concept, Some(created.id.clone()));

        // Deselect (view all)
        let deselect_req = SelectConceptRequest { id: None };
        let result = select_concept(&deselect_req).unwrap();
        assert!(result.current_concept.is_none());
    }

    #[test]
    #[serial]
    fn test_select_concept_not_found() {
        let _temp = setup();

        let req = SelectConceptRequest {
            id: Some("CON-999".to_string()),
        };
        let result = select_concept(&req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }

    #[test]
    #[serial]
    fn test_concept_task_count() {
        let _temp = setup();

        // Create a concept
        let req = CreateConceptRequest {
            name: "Concept with tasks".to_string(),
            description: None,
        };
        let created = create_concept(&req).unwrap();

        // Create tasks in the concept using new multi-concept API
        TaskRefs::create_with_concepts("Task 1", None, std::slice::from_ref(&created.id)).unwrap();
        TaskRefs::create_with_concepts("Task 2", None, std::slice::from_ref(&created.id)).unwrap();

        // Verify task count
        let concepts = list_concepts().unwrap();
        assert_eq!(concepts.concepts.len(), 1);
        assert_eq!(concepts.concepts[0].task_count, 2);
    }
}

// =============================================================================
// HANDLER TESTS - TASK UPDATE
// =============================================================================

mod task_update_handler_tests {
    use super::*;
    use noslop::api::{UpdateTaskRequest, get_task, update_task};

    #[test]
    #[serial]
    fn test_update_task_description() {
        let _temp = setup();

        let id = TaskRefs::create("My Task", None).unwrap();

        // Add a description
        let req = UpdateTaskRequest {
            description: Some("This is a detailed description".to_string()),
            concepts: None,
        };
        let result = update_task(&id, &req).unwrap();
        assert_eq!(result.description, Some("This is a detailed description".to_string()));

        // Verify it persisted
        let task = get_task(&id).unwrap();
        assert_eq!(task.description, Some("This is a detailed description".to_string()));
    }

    #[test]
    #[serial]
    fn test_update_task_no_change_when_description_none() {
        let _temp = setup();

        // Create task with description
        let id = TaskRefs::create("My Task", None).unwrap();
        TaskRefs::set_description(&id, Some("Initial description")).unwrap();

        // Pass None to leave description unchanged (None means "don't update")
        let req = UpdateTaskRequest {
            description: None,
            concepts: None,
        };
        let result = update_task(&id, &req).unwrap();
        // Description should still be set since None means "don't update"
        assert_eq!(result.description, Some("Initial description".to_string()));
    }

    #[test]
    #[serial]
    fn test_update_task_not_found() {
        let _temp = setup();

        let req = UpdateTaskRequest {
            description: Some("test".to_string()),
            concepts: None,
        };
        let result = update_task("FAKE-999", &req);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), 404);
    }
}

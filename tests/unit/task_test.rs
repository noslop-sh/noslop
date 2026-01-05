//! Tests for task module

use noslop::models::{Priority, Task, TaskStatus};

// =============================================================================
// TASK STATUS TESTS
// =============================================================================

#[test]
fn test_task_status_from_str_pending() {
    assert_eq!("pending".parse::<TaskStatus>().unwrap(), TaskStatus::Pending);
    assert_eq!("PENDING".parse::<TaskStatus>().unwrap(), TaskStatus::Pending);
}

#[test]
fn test_task_status_from_str_in_progress() {
    assert_eq!("in_progress".parse::<TaskStatus>().unwrap(), TaskStatus::InProgress);
    assert_eq!("in-progress".parse::<TaskStatus>().unwrap(), TaskStatus::InProgress);
    assert_eq!("inprogress".parse::<TaskStatus>().unwrap(), TaskStatus::InProgress);
    assert_eq!("started".parse::<TaskStatus>().unwrap(), TaskStatus::InProgress);
}

#[test]
fn test_task_status_from_str_done() {
    assert_eq!("done".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
    assert_eq!("complete".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
    assert_eq!("completed".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
}

#[test]
fn test_task_status_from_str_invalid() {
    let result = "invalid".parse::<TaskStatus>();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid status"));
}

#[test]
fn test_task_status_display() {
    assert_eq!(TaskStatus::Pending.to_string(), "pending");
    assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
    assert_eq!(TaskStatus::Done.to_string(), "done");
}

#[test]
fn test_task_status_default() {
    assert_eq!(TaskStatus::default(), TaskStatus::Pending);
}

// =============================================================================
// PRIORITY TESTS
// =============================================================================

#[test]
fn test_priority_from_str_p0() {
    assert_eq!("p0".parse::<Priority>().unwrap(), Priority::P0);
    assert_eq!("0".parse::<Priority>().unwrap(), Priority::P0);
    assert_eq!("critical".parse::<Priority>().unwrap(), Priority::P0);
}

#[test]
fn test_priority_from_str_p1() {
    assert_eq!("p1".parse::<Priority>().unwrap(), Priority::P1);
    assert_eq!("1".parse::<Priority>().unwrap(), Priority::P1);
    assert_eq!("high".parse::<Priority>().unwrap(), Priority::P1);
}

#[test]
fn test_priority_from_str_p2() {
    assert_eq!("p2".parse::<Priority>().unwrap(), Priority::P2);
    assert_eq!("2".parse::<Priority>().unwrap(), Priority::P2);
    assert_eq!("medium".parse::<Priority>().unwrap(), Priority::P2);
    assert_eq!("med".parse::<Priority>().unwrap(), Priority::P2);
}

#[test]
fn test_priority_from_str_p3() {
    assert_eq!("p3".parse::<Priority>().unwrap(), Priority::P3);
    assert_eq!("3".parse::<Priority>().unwrap(), Priority::P3);
    assert_eq!("low".parse::<Priority>().unwrap(), Priority::P3);
}

#[test]
fn test_priority_from_str_invalid() {
    let result = "p5".parse::<Priority>();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid priority"));
}

#[test]
fn test_priority_display() {
    assert_eq!(Priority::P0.to_string(), "p0");
    assert_eq!(Priority::P1.to_string(), "p1");
    assert_eq!(Priority::P2.to_string(), "p2");
    assert_eq!(Priority::P3.to_string(), "p3");
}

#[test]
fn test_priority_default() {
    assert_eq!(Priority::default(), Priority::P1);
}

// =============================================================================
// TASK TESTS
// =============================================================================

#[test]
fn test_task_new() {
    let task = Task::new("TSK-1".to_string(), "Test task".to_string());

    assert_eq!(task.id, "TSK-1");
    assert_eq!(task.title, "Test task");
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.priority, Priority::P1);
    assert!(task.blocked_by.is_empty());
    assert!(task.notes.is_none());
    assert!(!task.created_at.is_empty());
}

#[test]
fn test_task_with_options() {
    let task = Task::with_options(
        "TSK-2".to_string(),
        "Complex task".to_string(),
        Priority::P0,
        vec!["TSK-1".to_string()],
        Some("Important notes".to_string()),
    );

    assert_eq!(task.id, "TSK-2");
    assert_eq!(task.title, "Complex task");
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.priority, Priority::P0);
    assert_eq!(task.blocked_by, vec!["TSK-1"]);
    assert_eq!(task.notes, Some("Important notes".to_string()));
}

#[test]
fn test_task_is_ready_no_blockers() {
    let task = Task::new("TSK-1".to_string(), "Test task".to_string());
    let all_tasks = vec![task.clone()];

    assert!(task.is_ready(&all_tasks));
}

#[test]
fn test_task_is_ready_with_done_blocker() {
    let mut blocker = Task::new("TSK-1".to_string(), "Blocker".to_string());
    blocker.status = TaskStatus::Done;

    let task = Task::with_options(
        "TSK-2".to_string(),
        "Blocked task".to_string(),
        Priority::P1,
        vec!["TSK-1".to_string()],
        None,
    );

    let all_tasks = vec![blocker, task.clone()];
    assert!(task.is_ready(&all_tasks));
}

#[test]
fn test_task_is_ready_with_pending_blocker() {
    let blocker = Task::new("TSK-1".to_string(), "Blocker".to_string());

    let task = Task::with_options(
        "TSK-2".to_string(),
        "Blocked task".to_string(),
        Priority::P1,
        vec!["TSK-1".to_string()],
        None,
    );

    let all_tasks = vec![blocker, task.clone()];
    assert!(!task.is_ready(&all_tasks));
}

#[test]
fn test_task_is_ready_not_pending() {
    let mut task = Task::new("TSK-1".to_string(), "Test task".to_string());
    task.status = TaskStatus::InProgress;

    let all_tasks = vec![task.clone()];
    assert!(!task.is_ready(&all_tasks));
}

#[test]
fn test_task_is_ready_missing_blocker() {
    // If blocker doesn't exist, task is considered ready
    let task = Task::with_options(
        "TSK-2".to_string(),
        "Blocked task".to_string(),
        Priority::P1,
        vec!["TSK-MISSING".to_string()],
        None,
    );

    let all_tasks = vec![task.clone()];
    assert!(task.is_ready(&all_tasks));
}

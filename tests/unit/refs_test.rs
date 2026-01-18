//! Tests for refs-based task storage

use noslop::storage::TaskRefs;
use serial_test::serial;
use tempfile::TempDir;

fn setup() -> TempDir {
    let temp = TempDir::new().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    temp
}

// =============================================================================
// LIST AND BASIC OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_create_and_list() {
    let _temp = setup();

    // Initially empty
    let tasks = TaskRefs::list().unwrap();
    assert!(tasks.is_empty());

    // Create tasks
    let id1 = TaskRefs::create("Task one", None).unwrap();
    let id2 = TaskRefs::create("Task two", None).unwrap();
    let id3 = TaskRefs::create("Task three", Some("p0")).unwrap();

    // List should return all
    let tasks = TaskRefs::list().unwrap();
    assert_eq!(tasks.len(), 3);

    // Check IDs are sequential
    assert!(id1.ends_with("-1"));
    assert!(id2.ends_with("-2"));
    assert!(id3.ends_with("-3"));

    // Check priority was set
    let (_, task3) = tasks.iter().find(|(id, _)| id == &id3).unwrap();
    assert_eq!(task3.priority, "p0");
}

#[test]
#[serial]
fn test_refs_delete_task() {
    let _temp = setup();

    let id = TaskRefs::create("To be deleted", None).unwrap();
    assert!(TaskRefs::get(&id).unwrap().is_some());

    let deleted = TaskRefs::delete(&id).unwrap();
    assert!(deleted);

    // Should be gone
    assert!(TaskRefs::get(&id).unwrap().is_none());

    // Delete non-existent should return false
    let deleted_again = TaskRefs::delete(&id).unwrap();
    assert!(!deleted_again);
}

#[test]
#[serial]
fn test_refs_delete_clears_current() {
    let _temp = setup();

    let id = TaskRefs::create("Current task", None).unwrap();
    TaskRefs::set_current(&id).unwrap();

    assert_eq!(TaskRefs::current().unwrap(), Some(id.clone()));

    // Deleting current task should clear HEAD
    TaskRefs::delete(&id).unwrap();

    assert!(TaskRefs::current().unwrap().is_none());
}

// =============================================================================
// STATUS OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_set_status() {
    let _temp = setup();

    let id = TaskRefs::create("Status test", None).unwrap();

    // Initially backlog
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "backlog");

    // Set to pending
    TaskRefs::set_status(&id, "pending").unwrap();
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "pending");

    // Set to in_progress
    TaskRefs::set_status(&id, "in_progress").unwrap();
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "in_progress");

    // Set to done
    TaskRefs::set_status(&id, "done").unwrap();
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "done");

    // Non-existent task returns false
    let result = TaskRefs::set_status("FAKE-999", "done").unwrap();
    assert!(!result);
}

// =============================================================================
// PENDING UNBLOCKED AND NEXT OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_next_pending_unblocked_empty() {
    let _temp = setup();

    // No tasks
    assert!(TaskRefs::next_pending_unblocked().unwrap().is_none());

    // All done
    let id = TaskRefs::create("Done task", None).unwrap();
    TaskRefs::set_status(&id, "done").unwrap();
    assert!(TaskRefs::next_pending_unblocked().unwrap().is_none());
}

#[test]
#[serial]
fn test_refs_next_pending_unblocked_with_priority() {
    let _temp = setup();

    // Create tasks with different priorities and set them to pending
    let id_p2 = TaskRefs::create("Low priority", Some("p2")).unwrap();
    let id_p0 = TaskRefs::create("High priority", Some("p0")).unwrap();
    let id_p1 = TaskRefs::create("Medium priority", Some("p1")).unwrap();

    // Move all to pending (backlog tasks aren't actionable)
    TaskRefs::set_status(&id_p2, "pending").unwrap();
    TaskRefs::set_status(&id_p0, "pending").unwrap();
    TaskRefs::set_status(&id_p1, "pending").unwrap();

    // next_pending_unblocked should return highest priority (p0) first
    let (next_id, next_task) = TaskRefs::next_pending_unblocked().unwrap().unwrap();
    assert_eq!(next_id, id_p0);
    assert_eq!(next_task.priority, "p0");

    // Mark p0 as done, next should be p1
    TaskRefs::set_status(&id_p0, "done").unwrap();
    let (next_id, _) = TaskRefs::next_pending_unblocked().unwrap().unwrap();
    assert_eq!(next_id, id_p1);

    // Mark p1 as done, next should be p2
    TaskRefs::set_status(&id_p1, "done").unwrap();
    let (next_id, _) = TaskRefs::next_pending_unblocked().unwrap().unwrap();
    assert_eq!(next_id, id_p2);
}

#[test]
#[serial]
fn test_refs_list_pending_unblocked() {
    let _temp = setup();

    let id1 = TaskRefs::create("Pending 1", None).unwrap();
    let id2 = TaskRefs::create("Pending 2", None).unwrap();
    let id3 = TaskRefs::create("In progress", None).unwrap();
    let id4 = TaskRefs::create("Done", None).unwrap();

    // Set id1 and id2 to pending, others have different statuses
    TaskRefs::set_status(&id1, "pending").unwrap();
    TaskRefs::set_status(&id2, "pending").unwrap();
    TaskRefs::set_status(&id3, "in_progress").unwrap();
    TaskRefs::set_status(&id4, "done").unwrap();

    let pending_unblocked = TaskRefs::list_pending_unblocked().unwrap();
    assert_eq!(pending_unblocked.len(), 2);

    let ids: Vec<_> = pending_unblocked.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&id1.as_str()));
    assert!(ids.contains(&id2.as_str()));
}

// =============================================================================
// BLOCKER OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_add_remove_blocker() {
    let _temp = setup();

    let id1 = TaskRefs::create("Blocked task", None).unwrap();
    let id2 = TaskRefs::create("Blocker", None).unwrap();

    // Initially no blockers
    let task = TaskRefs::get(&id1).unwrap().unwrap();
    assert!(task.blocked_by.is_empty());

    // Add blocker
    TaskRefs::add_blocker(&id1, &id2).unwrap();
    let task = TaskRefs::get(&id1).unwrap().unwrap();
    assert_eq!(task.blocked_by, vec![id2.clone()]);

    // Adding same blocker again shouldn't duplicate
    TaskRefs::add_blocker(&id1, &id2).unwrap();
    let task = TaskRefs::get(&id1).unwrap().unwrap();
    assert_eq!(task.blocked_by.len(), 1);

    // Remove blocker
    TaskRefs::remove_blocker(&id1, &id2).unwrap();
    let task = TaskRefs::get(&id1).unwrap().unwrap();
    assert!(task.blocked_by.is_empty());
}

#[test]
#[serial]
fn test_refs_task_is_blocked() {
    let _temp = setup();

    let blocker_id = TaskRefs::create("Blocker task", None).unwrap();
    let blocked_id = TaskRefs::create("Blocked task", None).unwrap();

    // Move blocked task to pending (backlog tasks can't be "ready")
    TaskRefs::set_status(&blocked_id, "pending").unwrap();
    // Blocker stays in backlog - still counts as not done
    TaskRefs::add_blocker(&blocked_id, &blocker_id).unwrap();

    // Get all tasks to check is_blocked
    let tasks = TaskRefs::list().unwrap();
    let (_, blocked_task) = tasks.iter().find(|(id, _)| id == &blocked_id).unwrap();

    // Should be blocked while blocker is not done
    assert!(blocked_task.is_blocked(&tasks));

    // Complete the blocker
    TaskRefs::set_status(&blocker_id, "done").unwrap();

    // Re-fetch to get updated state
    let tasks = TaskRefs::list().unwrap();
    let (_, blocked_task) = tasks.iter().find(|(id, _)| id == &blocked_id).unwrap();

    // Now should be unblocked
    assert!(!blocked_task.is_blocked(&tasks));
}

// =============================================================================
// ID GENERATION
// =============================================================================

#[test]
#[serial]
fn test_refs_next_id_generation() {
    let _temp = setup();

    // Default prefix is TSK
    let id1 = TaskRefs::create("First", None).unwrap();
    assert!(id1.starts_with("TSK-"));

    // IDs should increment
    let id2 = TaskRefs::create("Second", None).unwrap();
    let id3 = TaskRefs::create("Third", None).unwrap();

    // Extract numbers and verify they increment
    let num1: u32 = id1.split('-').next_back().unwrap().parse().unwrap();
    let num2: u32 = id2.split('-').next_back().unwrap().parse().unwrap();
    let num3: u32 = id3.split('-').next_back().unwrap().parse().unwrap();

    assert_eq!(num2, num1 + 1);
    assert_eq!(num3, num2 + 1);
}

#[test]
#[serial]
fn test_refs_prefix_from_config() {
    let _temp = setup();

    // Create config with custom prefix
    std::fs::write(
        ".noslop.toml",
        r#"
prefix = "PROJ"

[[check]]
target = "*.rs"
message = "Test check"
"#,
    )
    .unwrap();

    let id = TaskRefs::create("Custom prefix task", None).unwrap();
    assert!(id.starts_with("PROJ-"), "Expected PROJ- prefix, got: {}", id);
}

// =============================================================================
// START, COMPLETE, AND BACKLOG OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_start_sets_status_and_current() {
    let _temp = setup();

    let id = TaskRefs::create("Task to start", None).unwrap();

    // Initially backlog
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "backlog");
    assert!(TaskRefs::current().unwrap().is_none());

    // Start the task
    let started = TaskRefs::start(&id).unwrap();
    assert!(started);

    // Status should be in_progress
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "in_progress");

    // Current should be set
    assert_eq!(TaskRefs::current().unwrap(), Some(id));
}

#[test]
#[serial]
fn test_refs_start_not_found() {
    let _temp = setup();

    let started = TaskRefs::start("FAKE-999").unwrap();
    assert!(!started);
}

#[test]
#[serial]
fn test_refs_complete_sets_status_and_clears_current() {
    let _temp = setup();

    let id = TaskRefs::create("Task to complete", None).unwrap();
    TaskRefs::set_status(&id, "in_progress").unwrap();
    TaskRefs::set_current(&id).unwrap();

    // Complete the task
    let completed = TaskRefs::complete(&id).unwrap();
    assert!(completed);

    // Status should be done
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "done");

    // Current should be cleared
    assert!(TaskRefs::current().unwrap().is_none());
}

#[test]
#[serial]
fn test_refs_complete_not_found() {
    let _temp = setup();

    let completed = TaskRefs::complete("FAKE-999").unwrap();
    assert!(!completed);
}

#[test]
#[serial]
fn test_refs_move_to_backlog_unlinks_branch() {
    let _temp = setup();

    let id = TaskRefs::create("Task", None).unwrap();
    TaskRefs::set_status(&id, "pending").unwrap();
    TaskRefs::link_branch(&id, Some("feature/test")).unwrap();
    TaskRefs::set_current(&id).unwrap();

    // Verify initial state
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.branch, Some("feature/test".to_string()));

    // Move to backlog
    let moved = TaskRefs::move_to_backlog(&id).unwrap();
    assert!(moved);

    // Verify final state
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.status, "backlog");
    assert!(task.branch.is_none());
    assert!(TaskRefs::current().unwrap().is_none());
}

#[test]
#[serial]
fn test_refs_move_to_backlog_not_found() {
    let _temp = setup();

    let moved = TaskRefs::move_to_backlog("FAKE-999").unwrap();
    assert!(!moved);
}

// =============================================================================
// CONCEPT OPERATIONS
// =============================================================================

#[test]
#[serial]
fn test_refs_set_concepts() {
    let _temp = setup();

    let id = TaskRefs::create("Task with concepts", None).unwrap();

    // Initially no concepts
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert!(task.concepts.is_empty());

    // Set concepts
    let result = TaskRefs::set_concepts(&id, &["CON-1", "CON-2"]).unwrap();
    assert!(result);

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts, vec!["CON-1".to_string(), "CON-2".to_string()]);

    // Unset concepts
    let result = TaskRefs::set_concepts(&id, &[]).unwrap();
    assert!(result);

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert!(task.concepts.is_empty());
}

#[test]
#[serial]
fn test_refs_add_remove_concept() {
    let _temp = setup();

    let id = TaskRefs::create("Task", None).unwrap();

    // Add first concept
    let result = TaskRefs::add_concept(&id, "CON-1").unwrap();
    assert!(result);
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts, vec!["CON-1".to_string()]);

    // Add second concept
    TaskRefs::add_concept(&id, "CON-2").unwrap();
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts, vec!["CON-1".to_string(), "CON-2".to_string()]);

    // Adding duplicate should not add again
    TaskRefs::add_concept(&id, "CON-1").unwrap();
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts.len(), 2);

    // Remove concept
    let result = TaskRefs::remove_concept(&id, "CON-1").unwrap();
    assert!(result);
    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts, vec!["CON-2".to_string()]);
}

#[test]
#[serial]
fn test_refs_set_concepts_not_found() {
    let _temp = setup();

    let result = TaskRefs::set_concepts("FAKE-999", &["CON-1"]).unwrap();
    assert!(!result);
}

#[test]
#[serial]
fn test_refs_create_with_concepts() {
    let _temp = setup();

    let concepts = vec!["CON-1".to_string(), "CON-2".to_string()];
    let id = TaskRefs::create_with_concepts("Task in concepts", None, &concepts).unwrap();

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.concepts, concepts);
    assert_eq!(task.title, "Task in concepts");
}

#[test]
#[serial]
fn test_refs_create_with_empty_concepts() {
    let _temp = setup();

    let id = TaskRefs::create_with_concepts("Task without concept", None, &[]).unwrap();

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert!(task.concepts.is_empty());
}

#[test]
#[serial]
fn test_refs_list_by_concept() {
    let _temp = setup();

    // Create tasks with different concepts
    let id1 = TaskRefs::create_with_concepts("Task 1", None, &["CON-1".to_string()]).unwrap();
    let id2 =
        TaskRefs::create_with_concepts("Task 2", None, &["CON-1".to_string(), "CON-2".to_string()])
            .unwrap();
    let id3 = TaskRefs::create_with_concepts("Task 3", None, &["CON-2".to_string()]).unwrap();
    let id4 = TaskRefs::create("Task 4 (no concept)", None).unwrap();

    // List tasks in CON-1 (should include task 1 and 2)
    let con1_tasks = TaskRefs::list_by_concept(Some("CON-1")).unwrap();
    assert_eq!(con1_tasks.len(), 2);
    let con1_ids: Vec<_> = con1_tasks.iter().map(|(id, _)| id.as_str()).collect();
    assert!(con1_ids.contains(&id1.as_str()));
    assert!(con1_ids.contains(&id2.as_str()));

    // List tasks in CON-2 (should include task 2 and 3)
    let con2_tasks = TaskRefs::list_by_concept(Some("CON-2")).unwrap();
    assert_eq!(con2_tasks.len(), 2);
    let con2_ids: Vec<_> = con2_tasks.iter().map(|(id, _)| id.as_str()).collect();
    assert!(con2_ids.contains(&id2.as_str()));
    assert!(con2_ids.contains(&id3.as_str()));

    // List tasks with no concept
    let no_con_tasks = TaskRefs::list_by_concept(None).unwrap();
    assert_eq!(no_con_tasks.len(), 1);
    assert_eq!(no_con_tasks[0].0, id4);
}

#[test]
#[serial]
fn test_refs_has_concept() {
    let _temp = setup();

    let id =
        TaskRefs::create_with_concepts("Task", None, &["CON-1".to_string(), "CON-2".to_string()])
            .unwrap();

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert!(task.has_concept("CON-1"));
    assert!(task.has_concept("CON-2"));
    assert!(!task.has_concept("CON-3"));
}

#[test]
#[serial]
fn test_refs_primary_concept() {
    let _temp = setup();

    let id =
        TaskRefs::create_with_concepts("Task", None, &["CON-1".to_string(), "CON-2".to_string()])
            .unwrap();

    let task = TaskRefs::get(&id).unwrap().unwrap();
    assert_eq!(task.primary_concept(), Some("CON-1"));

    // Empty concepts returns None
    let id2 = TaskRefs::create("Task 2", None).unwrap();
    let task2 = TaskRefs::get(&id2).unwrap().unwrap();
    assert_eq!(task2.primary_concept(), None);
}

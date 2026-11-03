use clickup_tracker::{
    api::ClickUp,
    model::{TimeEntry, format_time, task_id},
};
mod support;
use support::server;

#[test]
fn running_time_uses_start_timestamp_and_never_goes_negative() {
    let entry: TimeEntry =
        serde_json::from_str(r#"{"id":"1","start":"100000","duration":"-100000","task":null}"#)
            .unwrap();
    assert_eq!(entry.elapsed_ms(165000), 65000);
    assert_eq!(entry.elapsed_ms(90000), 0);
    let stopped: TimeEntry =
        serde_json::from_str(r#"{"id":"1","start":"100000","duration":"65000"}"#).unwrap();
    assert_eq!(stopped.elapsed_ms(999999), 65000);
    assert_eq!(format_time(3661000), "01:01:01");
}

#[test]
fn accepts_numeric_duration_from_clickup_start_response() {
    let entry = serde_json::from_str::<TimeEntry>(
        r#"{"id":"timer","start":"1595289395842","duration":-53,"task":{"id":"abc","name":"Task"}}"#,
    );
    assert!(
        entry.is_ok(),
        "ClickUp documents duration as a JSON integer"
    );
    assert_eq!(entry.unwrap().elapsed_ms(1595289455842), 60000);
}

#[test]
fn accepts_task_links_but_rejects_path_injection() {
    assert_eq!(
        task_id(" https://app.clickup.com/t/86abc123?foo=bar ").unwrap(),
        "86abc123"
    );
    assert_eq!(task_id("86abc123").unwrap(), "86abc123");
    assert!(task_id("../user").is_err());
    assert!(task_id("https://evil.test/t/abc").is_err());
    assert!(task_id("").is_err());
}

#[test]
fn switch_stops_old_timer_before_starting_new_one() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"old","task":{"id":"a","name":"Old"},"start":"1000","duration":"-1000"}}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"New","team_id":"42"}"#,
        ),
        (
            "POST /api/v2/team/42/time_entries/stop ",
            200,
            r#"{"data":{}}"#,
        ),
        (
            "POST /api/v2/team/42/time_entries/start \n{\"tid\":\"b\"}",
            200,
            r#"{"data":{"id":"new","task":{"id":"b","name":"New"},"start":"2000","duration":"-2000"}}"#,
        ),
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"new","task":{"id":"b","name":"New"},"start":"2000","duration":-2000}}"#,
        ),
    ]);
    let entry = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap();
    assert_eq!(entry.task.unwrap().id, "b");
    handle.join().unwrap();
}

#[test]
fn successful_start_is_confirmed_even_with_a_different_response_shape() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":null}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"New","team_id":"42"}"#,
        ),
        (
            "POST /api/v2/team/42/time_entries/start \n{\"tid\":\"b\"}",
            200,
            r#"{"data":{"id":123,"start":2000,"duration":-2000}}"#,
        ),
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"123","task":{"id":"b","name":"New"},"start":"2000","duration":-2000}}"#,
        ),
    ]);
    let entry = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap();
    assert_eq!(entry.id, "123");
    handle.join().unwrap();
}

#[test]
fn accepted_stop_does_not_require_a_json_response_body() {
    let (base, handle) = server(vec![("POST /api/v2/team/42/time_entries/stop ", 200, "")]);
    ClickUp::new("pk_test", &base).unwrap().stop("42").unwrap();
    handle.join().unwrap();
}

#[test]
fn start_does_not_report_success_without_the_requested_running_task() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":null}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"New","team_id":"42"}"#,
        ),
        ("POST /api/v2/team/42/time_entries/start ", 200, "{}"),
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":null}"#,
        ),
    ]);
    let error = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap_err();
    assert!(error.to_string().contains("has not confirmed"));
    handle.join().unwrap();
}

#[test]
fn lost_start_response_is_success_when_the_requested_timer_is_confirmed() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":null}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"New","team_id":"42"}"#,
        ),
        ("POST /api/v2/team/42/time_entries/start ", 502, "{}"),
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"123","task":{"id":"b","name":"New"},"start":"2000","duration":-2000}}"#,
        ),
    ]);
    assert_eq!(
        ClickUp::new("pk_test", &base)
            .unwrap()
            .switch_to("42", "b")
            .unwrap()
            .id,
        "123"
    );
    handle.join().unwrap();
}

#[test]
fn same_task_does_not_restart_a_running_timer() {
    let (base, handle) = server(vec![(
        "GET /api/v2/team/42/time_entries/current ",
        200,
        r#"{"data":{"id":"existing","task":{"id":"a","name":"Old"},"start":"1000","duration":"-1000"}}"#,
    )]);
    let entry = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "a")
        .unwrap();
    assert_eq!(entry.id, "existing");
    handle.join().unwrap();
}

#[test]
fn failed_stop_does_not_start_another_timer() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"old","task":{"id":"a","name":"Old"},"start":"1000","duration":"-1000"}}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"New","team_id":"42"}"#,
        ),
        (
            "POST /api/v2/team/42/time_entries/stop ",
            500,
            r#"{"err":"Internal error"}"#,
        ),
    ]);
    let error = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap_err();
    assert!(error.to_string().contains("500"));
    handle.join().unwrap();
}

#[test]
fn foreign_task_is_rejected_before_stopping_the_old_timer() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/time_entries/current ",
            200,
            r#"{"data":{"id":"old","task":{"id":"a","name":"Old"},"start":"1000","duration":-1000}}"#,
        ),
        (
            "GET /api/v2/task/b ",
            200,
            r#"{"id":"b","name":"Foreign","team_id":"99"}"#,
        ),
    ]);
    let error = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap_err();
    assert!(error.to_string().contains("workspace"));
    handle.join().unwrap();
}

#[test]
fn foreign_running_timer_is_not_stopped() {
    let (base, handle) = server(vec![(
        "GET /api/v2/team/42/time_entries/current ",
        200,
        r#"{"data":{"id":"old","wid":"99","task":{"id":"a","name":"Old"},"start":"1000","duration":-1000}}"#,
    )]);
    let error = ClickUp::new("pk_test", &base)
        .unwrap()
        .switch_to("42", "b")
        .unwrap_err();
    assert!(error.to_string().contains("workspace"));
    handle.join().unwrap();
}

#[test]
fn unauthorized_message_does_not_expose_token() {
    let (base, handle) = server(vec![(
        "GET /api/v2/user ",
        401,
        r#"{"err":"Unauthorized pk_test"}"#,
    )]);
    let error = ClickUp::new("pk_test", &base)
        .unwrap()
        .user()
        .unwrap_err()
        .to_string();
    assert!(error.contains("token"));
    assert!(!error.contains("pk_test"));
    handle.join().unwrap();
}

#[test]
fn no_running_timer_is_a_valid_state() {
    let (base, handle) = server(vec![(
        "GET /api/v2/team/42/time_entries/current ",
        200,
        r#"{"data":null}"#,
    )]);
    assert!(
        ClickUp::new("pk_test", &base)
            .unwrap()
            .current("42")
            .unwrap()
            .is_none()
    );
    handle.join().unwrap();
}

#[test]
fn task_pagination_includes_subtasks_and_assignee_filter() {
    // A full page requires requesting the next page even without last_page.
    let page = format!(
        r#"{{"tasks":[{}]}}"#,
        (0..100)
            .map(|i| format!(r#"{{"id":"{i}","name":"Task {i}"}}"#))
            .collect::<Vec<_>>()
            .join(",")
    );
    let page: &'static str = Box::leak(page.into_boxed_str());
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/task?page=0&subtasks=true&include_closed=false&assignees%5B%5D=7 ",
            200,
            page,
        ),
        (
            "GET /api/v2/team/42/task?page=1&subtasks=true&include_closed=false&assignees%5B%5D=7 ",
            200,
            r#"{"tasks":[{"id":"last","name":"Last task"}],"last_page":true}"#,
        ),
    ]);
    let tasks = ClickUp::new("pk_test", &base)
        .unwrap()
        .tasks("42", 7)
        .unwrap();
    assert_eq!(tasks.len(), 101);
    assert_eq!(tasks[100].id, "last");
    handle.join().unwrap();
}

#[test]
fn create_task_sends_a_trimmed_name_and_assigns_it_to_the_user() {
    let (base, handle) = server(vec![(
        "POST /api/v2/list/123/task \n{\"name\":\"New task\",\"assignees\":[7]}",
        200,
        r#"{"id":"new","name":"New task","team_id":"42","list":{"id":"123","name":"Tasks"}}"#,
    )]);
    let api = ClickUp::new("pk_test", &base).unwrap();
    let task = api.create_task("123", "  New task  ", 7).unwrap();
    assert_eq!(task.name, "New task");
    handle.join().unwrap();
}

#[test]
fn blank_task_name_is_rejected_without_a_network_write() {
    let api = ClickUp::new("pk_test", "http://127.0.0.1:1").unwrap();
    let error = api.create_task("123", "  ", 7).unwrap_err().to_string();
    assert!(error.contains("task name"));
}

#[test]
fn list_picker_includes_folderless_and_folder_lists() {
    let (base, handle) = server(vec![
        (
            "GET /api/v2/team/42/space?archived=false ",
            200,
            r#"{"spaces":[{"id":"s1","name":"Work"}]}"#,
        ),
        (
            "GET /api/v2/space/s1/list?archived=false ",
            200,
            r#"{"lists":[{"id":"1","name":"Inbox"}]}"#,
        ),
        (
            "GET /api/v2/space/s1/folder?archived=false ",
            200,
            r#"{"folders":[{"id":"f1","name":"Project","lists":[{"id":"2","name":"Tasks"}]}]}"#,
        ),
    ]);
    let lists = ClickUp::new("pk_test", &base).unwrap().lists("42").unwrap();
    assert_eq!(lists.len(), 2);
    assert!(
        lists
            .iter()
            .any(|l| l.id == "2" && l.location == "Work / Project")
    );
    handle.join().unwrap();
}

#[test]
fn summary_fetches_the_authenticated_users_entries_for_the_requested_range() {
    let (base, handle) = server(vec![(
        "GET /api/v2/team/42/time_entries?start_date=1000&end_date=9000 ",
        200,
        r#"{"data":[{"id":"1","start":"2000","duration":"3600000","task":null}]}"#,
    )]);
    let entries = ClickUp::new("pk_test", &base)
        .unwrap()
        .time_entries("42", 1000, 9000)
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].elapsed_ms(9000), 3600000);
    handle.join().unwrap();
}

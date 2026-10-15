//! Settled failed threads can reload settings after the last subscriber leaves.

use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use codex_app_server_protocol::AskForApproval;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadUnsubscribeParams;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;

#[tokio::test]
async fn unsubscribed_failed_thread_resume_applies_overrides() -> Result<()> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": {"message": "synthetic rate limit", "type": "rate_limit_exceeded"}
        })))
        .mount(&server)
        .await;
    let home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri())
        .with_root_config("thread_unload_delay_secs = 3600\napprovals_reviewer = \"user\"")
        .write(home.path())?;
    let mut app = TestAppServer::builder()
        .with_codex_home(home.path())
        .without_auto_env()
        .build_initialized()
        .await?;
    let id = app
        .send_thread_start_request(ThreadStartParams::default())
        .await?;
    let started: ThreadStartResponse = app.read_response(id).await?;
    let thread_id = started.thread.id;
    let id = app
        .send_turn_start_request(TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![UserInput::Text {
                text: "synthetic failure".to_string(),
                text_elements: vec![],
            }],
            ..Default::default()
        })
        .await?;
    let _: TurnStartResponse = app.read_response(id).await?;
    let notification = app
        .read_stream_until_notification_message("turn/completed")
        .await?;
    let completed: TurnCompletedNotification =
        serde_json::from_value(notification.params.unwrap())?;
    assert_eq!(completed.turn.status, TurnStatus::Failed);
    let id = app
        .send_thread_read_request(ThreadReadParams {
            thread_id: thread_id.clone(),
            include_turns: false,
        })
        .await?;
    let read: ThreadReadResponse = app.read_response(id).await?;
    assert_eq!(read.thread.status, ThreadStatus::SystemError);

    let params = ThreadResumeParams {
        thread_id: thread_id.clone(),
        approval_policy: Some(AskForApproval::OnRequest),
        exclude_turns: true,
        ..Default::default()
    };
    let id = app.send_thread_resume_request(params.clone()).await?;
    let subscribed: ThreadResumeResponse = app.read_response(id).await?;
    assert_eq!(subscribed.approval_policy, AskForApproval::Never);
    let id = app
        .send_thread_unsubscribe_request(ThreadUnsubscribeParams {
            thread_id: thread_id.clone(),
        })
        .await?;
    let unsubscribed = app
        .read_stream_until_response_message(RequestId::Integer(id))
        .await?;
    assert_eq!(unsubscribed.result["status"], "unsubscribed");
    let id = app.send_thread_resume_request(params).await?;
    let resumed: ThreadResumeResponse = app.read_response(id).await?;
    assert_eq!(resumed.approval_policy, AskForApproval::OnRequest);
    assert_eq!(resumed.thread.id, thread_id);
    let id = app
        .send_thread_read_request(ThreadReadParams {
            thread_id,
            include_turns: true,
        })
        .await?;
    let history: ThreadReadResponse = app.read_response(id).await?;
    assert_eq!(history.thread.turns.len(), 1);
    assert_eq!(history.thread.turns[0].status, TurnStatus::Failed);
    Ok(())
}

use chatgpt2api::{
    config::AppConfig,
    sse::{responses_sse_to_chat_sse, responses_sse_to_response_json},
};

#[test]
fn sse_collapses_completed_response_to_json() {
    let response = responses_sse_to_response_json(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"pong\"}\n\
         \n\
         data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"pong\"}]}]}}\n\n",
    )
    .unwrap();

    assert_eq!(response["id"], "resp_1");
    assert_eq!(response["output"][0]["content"][0]["text"], "pong");
}

#[test]
fn sse_fills_empty_completed_response_from_done_item() {
    let response = responses_sse_to_response_json(
        "data: {\"type\":\"response.output_item.done\",\"item\":{\"content\":[{\"type\":\"output_text\",\"text\":\"pong\"}]}}\n\
         \n\
         data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"output\":[]}}\n\n",
    )
    .unwrap();

    assert_eq!(response["id"], "resp_1");
    assert_eq!(response["output"][0]["content"][0]["text"], "pong");
}

#[test]
fn sse_does_not_duplicate_completed_text_from_done_events() {
    let response = responses_sse_to_response_json(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"pong\"}\n\
         \n\
         data: {\"type\":\"response.output_text.done\",\"text\":\"pong\"}\n\
         \n\
         data: {\"type\":\"response.content_part.done\",\"part\":{\"text\":\"pong\"}}\n\
         \n\
         data: {\"type\":\"response.output_item.done\",\"item\":{\"content\":[{\"type\":\"output_text\",\"text\":\"pong\"}]}}\n\
         \n\
         data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"output\":[]}}\n\n",
    )
    .unwrap();

    assert_eq!(response["id"], "resp_1");
    assert_eq!(response["output"][0]["content"][0]["text"], "pong");
}

#[test]
fn sse_translates_responses_deltas_to_openai_chat_chunks() {
    let config = AppConfig::default();
    let chunks = responses_sse_to_chat_sse(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\
         \n\
         data: {\"type\":\"response.completed\"}\n\n",
        &config,
    )
    .unwrap();

    assert!(chunks[0].contains(r#""content":"hello""#));
    assert_eq!(chunks[1], "data: [DONE]\n\n");
}

#[test]
fn sse_emits_reasoning_think_tags_when_configured() {
    let mut config = AppConfig::default();
    config.reasoning.compat = "think_tags".to_string();

    let chunks = responses_sse_to_chat_sse(
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"delta\":\"because\"}\n\n",
        &config,
    )
    .unwrap();

    assert!(chunks[0].contains("<think>because</think>"));
}

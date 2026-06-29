use chatgpt2api::{config::AppConfig, sse::responses_sse_to_chat_sse};

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

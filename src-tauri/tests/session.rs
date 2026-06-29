use chatgpt2api::session::session_id_for_prompt;

#[test]
fn session_id_is_stable_for_same_first_user_message() {
    let first = session_id_for_prompt("hello from the first user message");
    let second = session_id_for_prompt("hello from the first user message");

    assert_eq!(first, second);
    assert_ne!(first, session_id_for_prompt("different first user message"));
    assert!(first.starts_with("sess_"));
}

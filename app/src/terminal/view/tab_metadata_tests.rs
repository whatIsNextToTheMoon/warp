use super::custom_title_with_status;

#[test]
fn custom_title_preserves_live_spinner_without_changing_saved_name() {
    let name = "my task";
    for frame in ['\u{280b}', '\u{2819}', '\u{2733}', '\u{273b}', '\u{00b7}'] {
        assert_eq!(
            custom_title_with_status(&format!("{frame} agent task"), name),
            format!("{frame} {name}")
        );
    }
    assert_eq!(custom_title_with_status("Claude Code", name), name);
    assert_eq!(custom_title_with_status("", name), name);
    assert_eq!(custom_title_with_status("/workspace", name), name);
    assert_eq!(
        custom_title_with_status("\u{280b} working", "\u{280b} my task"),
        "\u{280b} my task"
    );
}

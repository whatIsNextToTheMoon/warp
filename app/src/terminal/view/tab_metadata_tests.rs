use super::{custom_title_with_status, strip_agent_spinner_prefixes};

#[test]
fn custom_title_preserves_live_spinner_without_changing_saved_name() {
    let name = "my task";
    // Only the frames that the real agents emit in the terminal title are treated as
    // spinners: Codex uses braille, Claude Code uses the quadrant circles and `✳`,
    // and `●` covers the voice indicator.
    for frame in [
        '\u{280b}', '\u{2819}', '\u{25d0}', '\u{25d1}', '\u{2733}', '\u{25cf}',
    ] {
        assert_eq!(
            custom_title_with_status(&format!("{frame} agent task"), name),
            format!("{frame} {name}")
        );
    }

    assert_eq!(custom_title_with_status("Claude Code", name), name);
    assert_eq!(custom_title_with_status("", name), name);
    assert_eq!(custom_title_with_status("/workspace", name), name);
    // Titles that merely start with something "spinner-ish" must not be copied.
    assert_eq!(custom_title_with_status("-zsh", name), name);
    assert_eq!(custom_title_with_status("\u{2736} agent task", name), name);
    assert_eq!(custom_title_with_status("\u{00b7} agent task", name), name);
}

#[test]
fn custom_title_does_not_stack_duplicate_spinner_frames() {
    // The saved name already carries the frame that was live when it was stored.
    assert_eq!(
        custom_title_with_status("\u{280b} working", "\u{280b} my task"),
        "\u{280b} my task"
    );
    // Frames rotate, so a stale frame must be replaced by the current one...
    assert_eq!(
        custom_title_with_status("\u{2819} working", "\u{280b} my task"),
        "\u{2819} my task"
    );
    // ...even when several frames were stored by an older build.
    assert_eq!(
        custom_title_with_status("\u{2819} working", "\u{280b} \u{2819} my task"),
        "\u{2819} my task"
    );
}

#[test]
fn strip_agent_spinner_prefixes_only_removes_frames() {
    assert_eq!(strip_agent_spinner_prefixes("\u{280b} my task"), "my task");
    assert_eq!(strip_agent_spinner_prefixes("\u{280b}\u{2819} my task"), "my task");
    assert_eq!(strip_agent_spinner_prefixes("\u{2819}  my task"), "my task");
    assert_eq!(strip_agent_spinner_prefixes("my task"), "my task");
    assert_eq!(strip_agent_spinner_prefixes("\u{280b}my task"), "\u{280b}my task");
}

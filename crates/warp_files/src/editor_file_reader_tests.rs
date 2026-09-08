use super::*;

#[test]
fn rejects_oversized_files_without_reading_or_modifying_them() {
    futures::executor::block_on(async {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(MAX_BYTES as u64 + 1).unwrap();
        for _ in 0..2 {
            assert!(matches!(
                read_content_for_editor(file.path()).await,
                Err(FileLoadError::EditorFileTooLarge)
            ));
        }
        assert_eq!(
            file.as_file().metadata().unwrap().len(),
            MAX_BYTES as u64 + 1
        );
    });
}

#[test]
fn bounds_layout_work_for_small_but_pathological_files() {
    assert!(matches!(
        validate_editor_content(&"x".repeat(MAX_LINE_BYTES + 1)),
        Err(FileLoadError::EditorLineTooLong)
    ));
    assert!(matches!(
        validate_editor_content(&"\n".repeat(MAX_LINES)),
        Err(FileLoadError::EditorTooManyLines)
    ));
    assert!(validate_editor_content(&"x".repeat(MAX_LINE_BYTES)).is_ok());
    assert!(validate_editor_content(&"\n".repeat(MAX_LINES - 1)).is_ok());
}

#[test]
fn reads_utf8_and_preserves_missing_file_semantics() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.txt");
        assert!(
            read_content_for_editor(&path)
                .await
                .unwrap_err()
                .is_not_found()
        );
        let content = "hello \u{4e2d}\u{6587}\n";
        async_fs::write(&path, content).await.unwrap();
        assert_eq!(read_content_for_editor(&path).await.unwrap(), content);
        assert!(matches!(
            read_content_for_editor(dir.path()).await,
            Err(FileLoadError::NotRegularFile)
        ));
    });
}

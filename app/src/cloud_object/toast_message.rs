use warpui::AppContext;

use super::{CloudObject, GenericStringObjectFormat, JsonObjectType, ObjectType};
use crate::server::cloud_objects::update_manager::{
    InitiatedBy, ObjectOperation, OperationSuccessType,
};

pub struct CloudObjectToastMessage;

impl CloudObjectToastMessage {
    pub fn toast_message(
        object: &dyn CloudObject,
        operation: &ObjectOperation,
        success_type: &OperationSuccessType,
        app: &AppContext,
    ) -> Option<String> {
        let chinese = crate::i18n::is_chinese_locale();

        let object_type_name_en = object.model_type_name();
        let object_name = if chinese {
            warpui::i18n::translate(object_type_name_en)
                .map(|t| t.into_owned())
                .unwrap_or_else(|| object_type_name_en.to_string())
        } else {
            object_type_name_en.to_string()
        };
        let object_name_lowercase = object_type_name_en.to_ascii_lowercase();

        match (object.object_type(), operation, success_type) {
            // We should only show toasts for creates initiated by the user, not by the system
            (
                _,
                ObjectOperation::Create {
                    initiated_by: InitiatedBy::User,
                },
                OperationSuccessType::Success,
            ) => {
                let containing_object_name = object.containing_object_name(app);
                if chinese {
                    Some(format!("{object_name} 已保存到 {containing_object_name}"))
                } else {
                    Some(format!("{object_name} saved to {containing_object_name}"))
                }
            }
            // notebooks intentionally do not have an update message, as they are updated
            // as the user types and so toasts would be VERY noisy
            (ObjectType::Notebook, ObjectOperation::Update, OperationSuccessType::Success) => None,
            (_, ObjectOperation::Update, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("{object_name} 已更新"))
                } else {
                    Some(format!("{object_name} updated"))
                }
            }
            (_, ObjectOperation::MoveToFolder, OperationSuccessType::Success)
            | (_, ObjectOperation::MoveToDrive, OperationSuccessType::Success) => {
                let containing_object_name = object.containing_object_name(app);
                if chinese {
                    Some(format!("{object_name} 已移动到 {containing_object_name}"))
                } else {
                    Some(format!("{object_name} moved to {containing_object_name}"))
                }
            }
            (_, ObjectOperation::Trash, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("{object_name} 已移入回收站"))
                } else {
                    Some(format!("{object_name} trashed"))
                }
            }
            (_, ObjectOperation::Untrash, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("{object_name} 已恢复"))
                } else {
                    Some(format!("{object_name} restored"))
                }
            }
            (_, ObjectOperation::Leave, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("已离开 {object_name}"))
                } else {
                    Some(format!("Left {object_name}"))
                }
            }
            (
                _,
                ObjectOperation::Create {
                    initiated_by: InitiatedBy::User,
                },
                OperationSuccessType::Failure,
            ) => {
                if chinese {
                    Some(format!("创建 {object_name} 失败"))
                } else {
                    Some(format!("Failed to create {object_name_lowercase}"))
                }
            }
            (
                _,
                ObjectOperation::Create {
                    initiated_by: InitiatedBy::User,
                },
                OperationSuccessType::Denied(message),
            ) => Some(message.to_string()),
            (_, ObjectOperation::Update, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("更新 {object_name} 失败"))
                } else {
                    Some(format!("Failed to update {object_name_lowercase}"))
                }
            }
            (_, ObjectOperation::MoveToFolder, OperationSuccessType::Failure)
            | (_, ObjectOperation::MoveToDrive, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("移动 {object_name} 失败"))
                } else {
                    Some(format!("Failed to move {object_name_lowercase}"))
                }
            }
            (_, ObjectOperation::Trash, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("移入回收站失败：{object_name}"))
                } else {
                    Some(format!("Failed to trash {object_name_lowercase}"))
                }
            }
            (_, ObjectOperation::Untrash, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("恢复失败：{object_name}"))
                } else {
                    Some(format!("Failed to restore {object_name_lowercase}"))
                }
            }
            // We should only show deletion failure toasts for user-initiated deletions.
            (
                _,
                ObjectOperation::Delete {
                    initiated_by: InitiatedBy::User,
                },
                OperationSuccessType::Failure,
            ) => {
                if chinese {
                    Some(format!("删除 {object_name} 失败"))
                } else {
                    Some(format!("Failed to delete {object_name_lowercase}"))
                }
            }
            (_, ObjectOperation::Leave, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("离开 {object_name} 失败"))
                } else {
                    Some(format!("Failed to leave {object_name}"))
                }
            }
            (ObjectType::Workflow, ObjectOperation::Update, OperationSuccessType::Rejection) => {
                if chinese {
                    Some("此工作流无法保存：你编辑期间发生了其他更改。".to_string())
                } else {
                    Some("This workflow could not be saved because changes were made while you were editing.".to_string())
                }
            }
            (
                ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
                    JsonObjectType::EnvVarCollection,
                )),
                ObjectOperation::Update,
                OperationSuccessType::Rejection,
            ) => {
                if chinese {
                    Some("环境变量无法保存：你编辑期间发生了其他更改。".to_string())
                } else {
                    Some("Environment variables could not be saved because changes were made while you were editing.".to_string())
                }
            }
            (
                ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
                    JsonObjectType::AIFact,
                )),
                ObjectOperation::Update,
                OperationSuccessType::Rejection,
            ) => {
                if chinese {
                    Some("规则无法保存：你编辑期间发生了其他更改。".to_string())
                } else {
                    Some(
                        "Rule could not be saved because changes were made while you were editing."
                            .to_string(),
                    )
                }
            }
            (_, ObjectOperation::TakeEditAccess, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("开始编辑 {object_name} 失败"))
                } else {
                    Some(format!("Failed to start editing {object_name_lowercase}"))
                }
            }
            (_, ObjectOperation::UpdatePermissions, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("{object_name} 权限已更新"))
                } else {
                    Some(format!(
                        "Successfully updated permissions for {object_name_lowercase}"
                    ))
                }
            }
            (_, ObjectOperation::UpdatePermissions, OperationSuccessType::Failure) => {
                if chinese {
                    Some(format!("更新 {object_name} 权限失败"))
                } else {
                    Some(format!(
                        "Failed to update permissions for {object_name_lowercase}"
                    ))
                }
            }
            _ => None,
        }
    }

    pub fn toast_deletion_confirm_message(
        num_objects: i32,
        operation: &ObjectOperation,
        success_type: &OperationSuccessType,
    ) -> Option<String> {
        let chinese = crate::i18n::is_chinese_locale();
        let count_objects_message = match num_objects {
            1 => {
                if chinese {
                    "1 个对象".to_string()
                } else {
                    "1 object".to_string()
                }
            }
            n => {
                if chinese {
                    format!("{n} 个对象")
                } else {
                    format!("{n} objects")
                }
            }
        };
        match (operation, success_type) {
            // We should only show deletion failure toasts for user-initiated deletions.
            (
                ObjectOperation::Delete {
                    initiated_by: InitiatedBy::User,
                },
                OperationSuccessType::Success,
            ) => {
                if chinese {
                    Some(format!("{count_objects_message} 已永久删除"))
                } else {
                    Some(format!("{count_objects_message} deleted forever"))
                }
            }
            (ObjectOperation::EmptyTrash, OperationSuccessType::Success) => {
                if chinese {
                    Some(format!("回收站已清空：{count_objects_message} 已永久删除"))
                } else {
                    Some(format!(
                        "Trash emptied: {count_objects_message} deleted forever"
                    ))
                }
            }
            (ObjectOperation::EmptyTrash, OperationSuccessType::Failure) => {
                if chinese {
                    Some("清空回收站失败".to_string())
                } else {
                    Some("Failed to empty trash".to_string())
                }
            }
            (ObjectOperation::EmptyTrash, OperationSuccessType::Rejection) => {
                if chinese {
                    Some("回收站中没有可清空的对象".to_string())
                } else {
                    Some("No objects in trash to empty".to_string())
                }
            }
            _ => None,
        }
    }
}

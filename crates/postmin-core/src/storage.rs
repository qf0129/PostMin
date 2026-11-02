use crate::model::{regenerate_ids, Item, Workspace, WORKSPACE_VERSION};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const EXPORT_FORMAT: &str = "postmin";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub exported_at_unix: u64,
    #[serde(default)]
    pub items: Vec<Item>,
}

fn default_format() -> String {
    EXPORT_FORMAT.to_string()
}

fn default_version() -> u32 {
    WORKSPACE_VERSION
}

impl ExportFile {
    pub fn new(items: Vec<Item>) -> Self {
        Self {
            format: EXPORT_FORMAT.to_string(),
            version: WORKSPACE_VERSION,
            exported_at_unix: now_unix(),
            items,
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        if let Ok(directory) = std::env::var("POSTMIN_DATA_DIR") {
            let directory = PathBuf::from(directory);
            if directory.extension().and_then(|value| value.to_str()) == Some("json") {
                return directory;
            }
            return directory.join("workspace.json");
        }
        let base = dirs::data_dir()
            .or_else(dirs::config_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("PostMin").join("workspace.json")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn directory(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn load(&self) -> Result<Option<Workspace>, String> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                let workspace: Workspace = serde_json::from_str(&contents).map_err(|error| {
                    format!("配置文件解析失败（{}）：{error}", self.path.display())
                })?;
                Ok(Some(workspace))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!(
                "读取配置文件失败（{}）：{error}",
                self.path.display()
            )),
        }
    }

    pub fn save(&self, workspace: &Workspace) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建配置目录失败（{}）：{error}", parent.display()))?;
        }
        let contents = serde_json::to_string_pretty(workspace)
            .map_err(|error| format!("序列化配置失败：{error}"))?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, contents).map_err(|error| format!("写入临时配置失败：{error}"))?;

        if self.path.exists() {
            let backup = self.path.with_extension("json.bak");
            let _ = fs::copy(&self.path, &backup);
            #[cfg(windows)]
            {
                fs::remove_file(&self.path)
                    .map_err(|error| format!("替换配置文件失败：{error}"))?;
            }
        }
        fs::rename(&temporary, &self.path).map_err(|error| format!("保存配置文件失败：{error}"))?;
        Ok(())
    }

    /// Copy the current file aside before a broken file is replaced.
    pub fn backup_corrupt(&self) -> Option<PathBuf> {
        if !self.path.exists() {
            return None;
        }
        let target = self
            .path
            .with_file_name(format!("workspace.corrupt.{}.json", now_unix()));
        fs::copy(&self.path, &target).ok().map(|_| target)
    }

    /// Move the previous file aside manually (used by the "restore backup" flow).
    pub fn move_aside(&self) -> Option<PathBuf> {
        if !self.path.exists() {
            return None;
        }
        let target = self
            .path
            .with_file_name(format!("workspace.previous.{}.json", now_unix()));
        fs::rename(&self.path, &target).ok().map(|_| target)
    }
}

pub fn export_items(items: &[Item]) -> Result<String, String> {
    let export = ExportFile::new(items.to_vec());
    serde_json::to_string_pretty(&export).map_err(|error| format!("导出失败：{error}"))
}

pub fn export_item(item: &Item) -> Result<String, String> {
    export_items(std::slice::from_ref(item))
}

pub fn import_items(input: &str) -> Result<Vec<Item>, String> {
    let trimmed = input.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return Err("导入内容为空".to_string());
    }

    let value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|error| format!("JSON 解析失败：{error}"))?;
    let mut items: Vec<Item> = if value.is_array() {
        serde_json::from_value::<Vec<Item>>(value)
            .map_err(|error| format!("无法识别的请求数组：{error}"))?
    } else if value.get("format").is_some() {
        let export: ExportFile = serde_json::from_value(value)
            .map_err(|error| format!("无法识别的导出文件：{error}"))?;
        if !export.format.is_empty() && export.format != EXPORT_FORMAT {
            return Err(format!("不支持的导入格式：{}", export.format));
        }
        export.items
    } else if value.get("items").is_some() {
        let workspace: Workspace = serde_json::from_value(value)
            .map_err(|error| format!("无法识别的 workspace：{error}"))?;
        workspace.items
    } else if value.get("type").is_some() {
        let item: Item = serde_json::from_value(value)
            .map_err(|error| format!("无法识别的目录/请求：{error}"))?;
        vec![item]
    } else {
        return Err(
            "无法识别的 JSON：需要 PostMin 导出的文件、目录/请求数组或 workspace".to_string(),
        );
    };

    if items.is_empty() {
        return Err("导入内容中没有接口".to_string());
    }
    normalize_imported(&mut items);
    regenerate_ids(&mut items);
    Ok(items)
}

fn normalize_imported(items: &mut [Item]) {
    for item in items {
        match item {
            Item::Folder(folder) => {
                if folder.name.trim().is_empty() {
                    folder.name = "未命名目录".to_string();
                }
                folder.expanded = true;
                normalize_imported(&mut folder.children);
            }
            Item::Request(request) => {
                if request.name.trim().is_empty() {
                    request.name = "未命名请求".to_string();
                }
                if request.method.trim().is_empty() {
                    request.method = "GET".to_string();
                }
                request.method = request.method.trim().to_ascii_uppercase();
                for header in &mut request.headers {
                    if header.key.trim().is_empty() {
                        header.enabled = false;
                    }
                }
            }
        }
    }
}

pub fn count_items(items: &[Item]) -> usize {
    items.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{count_requests, find_item, BodyKind, Folder, HeaderRow, SavedRequest};

    fn sample() -> Workspace {
        let mut folder = Folder::new("API");
        folder.children.push(Item::Request(SavedRequest {
            id: "fixed".to_string(),
            name: "Get user".to_string(),
            method: "GET".to_string(),
            url: "https://example.com/users/1".to_string(),
            headers: vec![HeaderRow::new("Accept", "application/json")],
            body: String::new(),
            body_kind: BodyKind::None,
            auto_name: false,
        }));
        Workspace {
            version: WORKSPACE_VERSION,
            settings: Default::default(),
            items: vec![Item::Folder(folder)],
            environments: Default::default(),
            active_environment: None,
            last_request_id: Some("fixed".to_string()),
        }
    }

    #[test]
    fn saves_and_loads_workspace() {
        let directory = tempfile::tempdir().expect("tempdir");
        let storage = Storage::new(directory.path().join("workspace.json"));
        assert!(storage.load().unwrap().is_none());

        let workspace = sample();
        storage.save(&workspace).expect("save");
        let loaded = storage.load().expect("load").expect("exists");
        assert_eq!(loaded, workspace);
        assert_eq!(count_requests(&loaded.items), 1);
        assert!(loaded.last_request_id.is_some());
    }

    #[test]
    fn exports_and_reimports_with_new_ids() {
        let workspace = sample();
        let json = export_items(&workspace.items).expect("export");
        assert!(json.contains("\"format\": \"postmin\""));

        let imported = import_items(&json).expect("import");
        assert_eq!(count_requests(&imported), 1);
        assert!(find_item(&imported, "fixed").is_none());
        let folder = match &imported[0] {
            Item::Folder(folder) => folder,
            _ => panic!("folder expected"),
        };
        assert_eq!(folder.children[0].name(), "Get user");
        assert!(folder.expanded);
    }

    #[test]
    fn imports_plain_arrays_and_single_items() {
        let request = Item::Request(SavedRequest {
            id: "a".to_string(),
            name: "A".to_string(),
            ..SavedRequest::new()
        });
        let array = serde_json::to_string(&vec![request.clone()]).unwrap();
        assert_eq!(import_items(&array).unwrap().len(), 1);

        let single = serde_json::to_string(&request).unwrap();
        assert_eq!(import_items(&single).unwrap().len(), 1);
        assert!(import_items("not json").is_err());
        assert!(import_items("[]").is_err());
    }

    #[test]
    fn backups_corrupt_files() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("workspace.json");
        std::fs::write(&path, "{broken").unwrap();
        let storage = Storage::new(&path);
        assert!(storage.load().is_err());
        let backup = storage.backup_corrupt().expect("backup");
        assert!(backup.exists());
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "{broken");
    }

    #[test]
    fn serializes_export_shape_for_manual_editing() {
        let json = export_items(&sample().items).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["format"], "postmin");
        assert!(value["items"][0]["children"].is_array());
        assert_eq!(value["items"][0]["type"], "folder");
        assert_eq!(value["items"][0]["children"][0]["type"], "request");
    }
}

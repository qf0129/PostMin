use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const WORKSPACE_VERSION: u32 = 1;

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_version() -> u32 {
    WORKSPACE_VERSION
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "深色",
            ThemeMode::Light => "浅色",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default = "default_environments")]
    pub environments: Vec<Environment>,
    #[serde(default)]
    pub active_environment: Option<String>,
    #[serde(default)]
    pub last_request_id: Option<String>,
}

impl Default for Workspace {
    fn default() -> Self {
        let environments = default_environments();
        let active_environment = environments
            .first()
            .map(|environment| environment.id.clone());
        Self {
            version: WORKSPACE_VERSION,
            settings: Settings::default(),
            items: Vec::new(),
            environments,
            active_environment,
            last_request_id: None,
        }
    }
}

impl Workspace {
    /// Make sure there is at least one environment and that the active id is valid.
    pub fn ensure_environments(&mut self) {
        if self.environments.is_empty() {
            self.environments = default_environments();
        }
        let valid = self.active_environment.as_deref().is_some_and(|id| {
            self.environments
                .iter()
                .any(|environment| environment.id == id)
        });
        if !valid {
            self.active_environment = self
                .environments
                .first()
                .map(|environment| environment.id.clone());
        }
    }

    pub fn active_environment(&self) -> Option<&Environment> {
        let id = self.active_environment.as_deref()?;
        self.environments
            .iter()
            .find(|environment| environment.id == id)
    }

    pub fn active_environment_mut(&mut self) -> Option<&mut Environment> {
        let id = self.active_environment.as_deref()?;
        self.environments
            .iter_mut()
            .find(|environment| environment.id == id)
    }
}

fn default_environments() -> Vec<Environment> {
    let mut test = Environment::new("test");
    test.variables.push(EnvironmentVariable {
        name: "baseUrl".to_string(),
        value: "https://httpbin.org".to_string(),
        description: "示例接口根地址".to_string(),
    });
    test.variables.push(EnvironmentVariable {
        name: "token".to_string(),
        value: "demo-token".to_string(),
        description: "示例鉴权令牌".to_string(),
    });
    vec![test]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default = "default_environment_name")]
    pub name: String,
    #[serde(default)]
    pub variables: Vec<EnvironmentVariable>,
}

fn default_environment_name() -> String {
    "新环境".to_string()
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            variables: Vec::new(),
        }
    }

    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            "未命名环境"
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub description: String,
}

impl EnvironmentVariable {
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default)]
    pub theme: ThemeMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            follow_redirects: true,
            theme: ThemeMode::Light,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    Folder(Folder),
    Request(SavedRequest),
}

impl Item {
    pub fn id(&self) -> &str {
        match self {
            Item::Folder(folder) => &folder.id,
            Item::Request(request) => &request.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Item::Folder(folder) => &folder.name,
            Item::Request(request) => &request.name,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, Item::Folder(_))
    }

    pub fn as_request(&self) -> Option<&SavedRequest> {
        match self {
            Item::Request(request) => Some(request),
            Item::Folder(_) => None,
        }
    }

    pub fn as_request_mut(&mut self) -> Option<&mut SavedRequest> {
        match self {
            Item::Request(request) => Some(request),
            Item::Folder(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default = "default_folder_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub expanded: bool,
    #[serde(default)]
    pub children: Vec<Item>,
}

fn default_folder_name() -> String {
    "新建目录".to_string()
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            expanded: true,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default = "default_request_name")]
    pub name: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderRow>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub body_kind: BodyKind,
    #[serde(default)]
    pub auto_name: bool,
}

fn default_request_name() -> String {
    "未命名请求".to_string()
}

fn default_method() -> String {
    "GET".to_string()
}

impl SavedRequest {
    pub fn new() -> Self {
        Self {
            id: new_id(),
            name: default_request_name(),
            method: default_method(),
            url: String::new(),
            headers: Vec::new(),
            body: String::new(),
            body_kind: BodyKind::None,
            auto_name: true,
        }
    }

    pub fn content_type(&self) -> Option<&'static str> {
        self.body_kind.content_type()
    }
}

impl Default for SavedRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HeaderRow {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub value: String,
}

impl HeaderRow {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            enabled: true,
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum BodyKind {
    #[default]
    None,
    Json,
    Xml,
    Text,
}

impl BodyKind {
    pub fn content_type(self) -> Option<&'static str> {
        match self {
            BodyKind::None => None,
            BodyKind::Json => Some("application/json; charset=utf-8"),
            BodyKind::Xml => Some("application/xml; charset=utf-8"),
            BodyKind::Text => Some("text/plain; charset=utf-8"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BodyKind::None => "无",
            BodyKind::Json => "JSON",
            BodyKind::Xml => "XML",
            BodyKind::Text => "文本",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentRef {
    Root,
    Folder(String),
}

pub fn find_item<'a>(items: &'a [Item], id: &str) -> Option<&'a Item> {
    for item in items {
        if item.id() == id {
            return Some(item);
        }
        if let Item::Folder(folder) = item {
            if let Some(found) = find_item(&folder.children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn find_item_mut<'a>(items: &'a mut [Item], id: &str) -> Option<&'a mut Item> {
    for item in items.iter_mut() {
        if item.id() == id {
            return Some(item);
        }
        if let Item::Folder(folder) = item {
            if let Some(found) = find_item_mut(&mut folder.children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn find_folder_mut<'a>(items: &'a mut [Item], id: &str) -> Option<&'a mut Folder> {
    for item in items.iter_mut() {
        if let Item::Folder(folder) = item {
            if folder.id == id {
                return Some(folder);
            }
            if let Some(found) = find_folder_mut(&mut folder.children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn find_parent(items: &[Item], target_id: &str) -> Option<ParentRef> {
    for item in items {
        if item.id() == target_id {
            return Some(ParentRef::Root);
        }
        if let Item::Folder(folder) = item {
            if folder.children.iter().any(|child| child.id() == target_id) {
                return Some(ParentRef::Folder(folder.id.clone()));
            }
            if let Some(found) = find_parent(&folder.children, target_id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn insert_item(items: &mut Vec<Item>, parent: Option<&str>, item: Item) -> bool {
    match parent {
        None => {
            items.push(item);
            true
        }
        Some(parent_id) => {
            if let Some(folder) = find_folder_mut(items, parent_id) {
                folder.children.push(item);
                folder.expanded = true;
                true
            } else {
                false
            }
        }
    }
}

pub fn insert_after(items: &mut Vec<Item>, target_id: &str, item: Item) -> bool {
    if let Some(position) = items
        .iter()
        .position(|candidate| candidate.id() == target_id)
    {
        items.insert(position + 1, item);
        return true;
    }
    for candidate in items.iter_mut() {
        if let Item::Folder(folder) = candidate {
            if insert_after(&mut folder.children, target_id, item.clone()) {
                return true;
            }
        }
    }
    false
}

pub fn remove_item(items: &mut Vec<Item>, id: &str) -> Option<Item> {
    if let Some(position) = items.iter().position(|item| item.id() == id) {
        return Some(items.remove(position));
    }
    for item in items.iter_mut() {
        if let Item::Folder(folder) = item {
            if let Some(found) = remove_item(&mut folder.children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn count_requests(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Request(_) => 1,
            Item::Folder(folder) => count_requests(&folder.children),
        })
        .sum()
}

pub fn regenerate_ids(items: &mut [Item]) {
    for item in items {
        match item {
            Item::Folder(folder) => {
                folder.id = new_id();
                regenerate_ids(&mut folder.children);
            }
            Item::Request(request) => {
                request.id = new_id();
            }
        }
    }
}

pub fn duplicate_item(item: &Item) -> Item {
    let mut cloned = item.clone();
    regenerate_ids(std::slice::from_mut(&mut cloned));
    match &mut cloned {
        Item::Folder(folder) => {
            folder.name = format!("{} 副本", folder.name);
        }
        Item::Request(request) => {
            request.name = format!("{} 副本", request.name);
        }
    }
    cloned
}

pub fn collect_request_ids(items: &[Item], out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::Request(request) => out.push(request.id.clone()),
            Item::Folder(folder) => collect_request_ids(&folder.children, out),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    Before(String),
    After(String),
    Into(String),
    RootEnd,
}

/// Move a folder or request to a new location in the tree.
///
/// Returns `false` without changing the tree when the move is invalid
/// (unknown ids, moving onto itself, or moving a folder into its own subtree).
pub fn move_item(items: &mut Vec<Item>, dragged_id: &str, target: &MoveTarget) -> bool {
    let Some(source) = find_item(items, dragged_id) else {
        return false;
    };

    let target_id = match target {
        MoveTarget::Before(id) | MoveTarget::After(id) | MoveTarget::Into(id) => Some(id.as_str()),
        MoveTarget::RootEnd => None,
    };

    if let Some(target_id) = target_id {
        if target_id == dragged_id {
            return false;
        }
        if let Item::Folder(folder) = source {
            if find_item(&folder.children, target_id).is_some() {
                return false;
            }
        }
        let Some(target_item) = find_item(items, target_id) else {
            return false;
        };
        if matches!(target, MoveTarget::Into(_)) && !target_item.is_folder() {
            return false;
        }
    }

    let Some(source) = remove_item(items, dragged_id) else {
        return false;
    };

    let inserted = match target {
        MoveTarget::Before(target_id) => insert_relative_or_root(items, target_id, source, false),
        MoveTarget::After(target_id) => insert_relative_or_root(items, target_id, source, true),
        MoveTarget::Into(target_id) => {
            if let Some(folder) = find_folder_mut(items, target_id) {
                folder.children.push(source);
                folder.expanded = true;
                true
            } else {
                items.push(source);
                false
            }
        }
        MoveTarget::RootEnd => {
            items.push(source);
            true
        }
    };

    inserted
}

fn insert_relative_or_root(
    items: &mut Vec<Item>,
    target_id: &str,
    item: Item,
    after: bool,
) -> bool {
    match insert_relative_once(items, target_id, item, after) {
        None => true,
        Some(item) => {
            items.push(item);
            false
        }
    }
}

fn insert_relative_once(
    items: &mut Vec<Item>,
    target_id: &str,
    item: Item,
    after: bool,
) -> Option<Item> {
    if let Some(position) = items
        .iter()
        .position(|candidate| candidate.id() == target_id)
    {
        items.insert(position + usize::from(after), item);
        return None;
    }
    let mut item = item;
    for candidate in items.iter_mut() {
        if let Item::Folder(folder) = candidate {
            item = insert_relative_once(&mut folder.children, target_id, item, after)?;
        }
    }
    Some(item)
}

pub fn sample_workspace() -> Workspace {
    let mut sample = Folder::new("示例接口");
    sample.expanded = true;
    sample.children.push(Item::Request(SavedRequest {
        id: new_id(),
        name: "GET 示例".to_string(),
        method: "GET".to_string(),
        url: "https://httpbin.org/get?from=postmin".to_string(),
        headers: vec![HeaderRow::new("Accept", "application/json")],
        body: String::new(),
        body_kind: BodyKind::None,
        auto_name: false,
    }));
    sample.children.push(Item::Request(SavedRequest {
        id: new_id(),
        name: "POST JSON 示例".to_string(),
        method: "POST".to_string(),
        url: "https://httpbin.org/post".to_string(),
        headers: vec![HeaderRow::new("Accept", "application/json")],
        body: "{\n  \"name\": \"PostMin\",\n  \"message\": \"Hello, API\"\n}".to_string(),
        body_kind: BodyKind::Json,
        auto_name: false,
    }));

    let mut workspace = Workspace::default();
    workspace.items.push(Item::Folder(sample));
    workspace
}

/// Derive a readable request name from a URL.
pub fn name_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let path_and_query = without_scheme.split_once('/').map(|(_, rest)| rest);
    let path = path_and_query.unwrap_or("");
    let path = path.split(['?', '#']).next().unwrap_or("");
    let last = path
        .split('/')
        .rfind(|segment| !segment.is_empty())
        .map(|segment| segment.to_string());
    if let Some(last) = last {
        return Some(last);
    }
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, name: &str) -> Item {
        Item::Request(SavedRequest {
            id: id.to_string(),
            name: name.to_string(),
            ..SavedRequest::new()
        })
    }

    #[test]
    fn finds_and_removes_nested_items() {
        let mut folder = Folder::new("root");
        folder.children.push(request("child", "child"));
        let mut items = vec![Item::Folder(folder), request("top", "top")];

        assert!(find_item(&items, "child").is_some());
        assert_eq!(
            find_parent(&items, "child"),
            Some(ParentRef::Folder(items[0].id().to_string()))
        );
        assert_eq!(find_parent(&items, "top"), Some(ParentRef::Root));

        let removed = remove_item(&mut items, "child").expect("child exists");
        assert_eq!(removed.name(), "child");
        assert!(find_item(&items, "child").is_none());
    }

    #[test]
    fn inserts_into_folder_and_after_item() {
        let folder = Folder::new("root");
        let folder_id = folder.id.clone();
        let mut items = vec![Item::Folder(folder)];
        assert!(insert_item(&mut items, Some(&folder_id), request("a", "a")));
        assert!(insert_after(&mut items, "a", request("b", "b")));
        let folder = match &items[0] {
            Item::Folder(folder) => folder,
            _ => panic!("folder expected"),
        };
        assert_eq!(folder.children.len(), 2);
        assert_eq!(folder.children[1].id(), "b");
        assert!(folder.expanded);
    }

    #[test]
    fn duplicate_regenerates_ids() {
        let mut folder = Folder::new("root");
        folder.children.push(request("child", "child"));
        let original = Item::Folder(folder);
        let copy = duplicate_item(&original);
        assert_ne!(copy.id(), original.id());
        let (orig_children, copy_children) = match (&original, &copy) {
            (Item::Folder(a), Item::Folder(b)) => (&a.children, &b.children),
            _ => panic!("folders expected"),
        };
        assert_ne!(orig_children[0].id(), copy_children[0].id());
        assert!(copy.name().ends_with("副本"));
        assert_eq!(count_requests(&[copy]), 1);
    }

    #[test]
    fn deserializes_missing_defaults() {
        let value = serde_json::json!({
            "items": [
                { "type": "request", "name": "test" }
            ]
        });
        let mut workspace: Workspace = serde_json::from_value(value).expect("valid workspace");
        assert_eq!(workspace.version, WORKSPACE_VERSION);
        assert_eq!(workspace.settings.timeout_secs, 30);
        assert_eq!(workspace.settings.theme, ThemeMode::Light);
        let request = workspace.items[0].as_request().expect("request");
        assert_eq!(request.method, "GET");
        assert!(!request.id.is_empty());
        assert!(!request.auto_name);
        assert_eq!(workspace.environments.len(), 1);
        assert_eq!(workspace.environments[0].name, "test");
        assert_eq!(workspace.environments[0].variables[0].name, "baseUrl");
        workspace.ensure_environments();
        assert_eq!(
            workspace.active_environment.as_deref(),
            Some(workspace.environments[0].id.as_str())
        );
    }

    #[test]
    fn serializes_environments() {
        let mut workspace = Workspace::default();
        workspace.environments[0]
            .variables
            .push(EnvironmentVariable::new("region", "cn", "部署区域"));
        let encoded = serde_json::to_string(&workspace).expect("serialize");
        let decoded: Workspace = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded.environments, workspace.environments);
        assert_eq!(decoded.active_environment, workspace.active_environment);
        assert_eq!(decoded.environments[0].display_name(), "test");
    }

    #[test]
    fn parses_theme_mode_settings() {
        let workspace: Workspace = serde_json::from_value(serde_json::json!({
            "settings": { "theme": "light" },
            "items": []
        }))
        .expect("valid workspace");
        assert_eq!(workspace.settings.theme, ThemeMode::Light);
        let encoded = serde_json::to_value(&workspace.settings).expect("serialize");
        assert_eq!(encoded["theme"], "light");
    }

    #[test]
    fn moves_items_with_drag_and_drop_targets() {
        let mut inner = Folder::new("inner");
        inner.id = "inner".to_string();
        inner.children.push(request("b", "b"));
        let mut outer = Folder::new("outer");
        outer.id = "outer".to_string();
        outer.children.push(request("a", "a"));
        outer.children.push(Item::Folder(inner));
        let mut items = vec![Item::Folder(outer), request("c", "c")];

        assert!(move_item(
            &mut items,
            "a",
            &MoveTarget::After("c".to_string())
        ));
        assert_eq!(
            items.iter().map(Item::id).collect::<Vec<_>>(),
            vec!["outer".to_string(), "c".to_string(), "a".to_string()]
        );

        assert!(move_item(
            &mut items,
            "b",
            &MoveTarget::Into("outer".to_string())
        ));
        let outer = match &items[0] {
            Item::Folder(folder) => folder,
            _ => panic!("folder expected"),
        };
        assert!(outer.children.iter().any(|child| child.id() == "b"));
        assert!(outer.expanded);

        assert!(!move_item(
            &mut items,
            "outer",
            &MoveTarget::Into("inner".to_string())
        ));
        assert!(!move_item(
            &mut items,
            "a",
            &MoveTarget::Into("a".to_string())
        ));
        assert!(!move_item(
            &mut items,
            "a",
            &MoveTarget::Into("c".to_string())
        ));
        assert!(!move_item(&mut items, "missing", &MoveTarget::RootEnd));
        assert!(move_item(&mut items, "a", &MoveTarget::RootEnd));
        assert_eq!(items.last().map(Item::id), Some("a"));
    }

    #[test]
    fn derives_names_from_urls() {
        assert_eq!(
            name_from_url("https://api.example.com/v1/users/42?x=1"),
            Some("42".to_string())
        );
        assert_eq!(
            name_from_url("https://api.example.com"),
            Some("api.example.com".to_string())
        );
        assert_eq!(name_from_url("   "), None);
    }
}

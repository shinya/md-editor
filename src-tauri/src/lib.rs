use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, Window};

// 変数の定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
}

// 変数セット
#[derive(Debug, Serialize, Deserialize)]
pub struct VariableSet {
    pub variables: Vec<Variable>,
}

// ウィンドウ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub maximized: bool,
    pub minimized: bool,
}

// ディスプレイ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub scale_factor: f64,
    pub is_primary: bool,
}

// システム情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub displays: Vec<DisplayInfo>,
    pub primary_display_index: usize,
}

// 変数処理器
pub struct VariableProcessor {
    global_variables: Mutex<HashMap<String, String>>,
}

impl VariableProcessor {
    pub fn new() -> Self {
        Self {
            global_variables: Mutex::new(HashMap::new()),
        }
    }

    // グローバル変数を設定
    pub fn set_global_variable(&self, name: String, value: String) {
        let mut vars = self.global_variables.lock().unwrap();
        vars.insert(name, value);
    }

    // グローバル変数を取得
    pub fn get_global_variable(&self, name: &str) -> Option<String> {
        let vars = self.global_variables.lock().unwrap();
        vars.get(name).cloned()
    }

    // すべてのグローバル変数を取得
    pub fn get_all_global_variables(&self) -> HashMap<String, String> {
        let vars = self.global_variables.lock().unwrap();
        vars.clone()
    }

    // Markdownから変数定義を抽出
    pub fn parse_variables_from_markdown(&self, content: &str) -> (Vec<Variable>, String) {
        let mut variables = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut processed_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();

            // 変数定義のパターンをチェック
            if trimmed.starts_with("<!-- @var ") && trimmed.ends_with(" -->") {
                // <!-- @var name: value --> の形式
                let var_content = trimmed
                    .strip_prefix("<!-- @var ")
                    .unwrap()
                    .strip_suffix(" -->")
                    .unwrap();

                if let Some(colon_index) = var_content.find(':') {
                    let name = var_content[..colon_index].trim().to_string();
                    let value = var_content[colon_index + 1..].trim().to_string();
                    variables.push(Variable { name, value });
                }
            } else if trimmed.starts_with("<!-- @include:") && trimmed.ends_with(" -->") {
                // <!-- @include: filename --> の形式（将来実装）
                // 現在はスキップ
            } else {
                processed_lines.push(line);
            }
        }

        (variables, processed_lines.join("\n"))
    }

    // Markdownコンテンツ内の変数を展開
    pub fn process_variables(&self, content: &str) -> String {
        // ファイル内の変数定義を抽出
        let (file_variables, processed_content) = self.parse_variables_from_markdown(content);

        // ファイル内変数をマップに変換
        let mut file_var_map = HashMap::new();
        for v in file_variables {
            file_var_map.insert(v.name, v.value);
        }

        // 変数展開の正規表現
        let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();

        // 変数を展開
        let result = re.replace_all(&processed_content, |caps: &regex::Captures| {
            let var_name = caps.get(1).unwrap().as_str().trim();

            // ファイル内変数を優先、次にグローバル変数
            if let Some(value) = file_var_map.get(var_name) {
                return value.clone();
            }
            if let Some(value) = self.get_global_variable(var_name) {
                return value;
            }

            // 変数が見つからない場合は元の文字列を返す
            caps[0].to_string()
        });

        result.to_string()
    }

    // YAMLファイルから変数を読み込み
    pub fn load_variables_from_yaml(&self, yaml_content: &str) -> Result<()> {
        let var_set: VariableSet = serde_yaml::from_str(yaml_content)?;
        let mut vars = self.global_variables.lock().unwrap();

        for v in var_set.variables {
            vars.insert(v.name, v.value);
        }

        Ok(())
    }

    // 変数をYAML形式でエクスポート
    pub fn export_variables_to_yaml(&self) -> Result<String> {
        let vars = self.get_all_global_variables();
        let variables: Vec<Variable> = vars
            .into_iter()
            .map(|(name, value)| Variable { name, value })
            .collect();

        let var_set = VariableSet { variables };
        let yaml_content = serde_yaml::to_string(&var_set)?;

        Ok(yaml_content)
    }
}

// グローバル変数処理器のインスタンス
lazy_static::lazy_static! {
    static ref VARIABLE_PROCESSOR: VariableProcessor = VariableProcessor::new();
}

// Tauriコマンド: グローバル変数を設定
#[tauri::command]
fn set_global_variable(name: String, value: String) -> Result<(), String> {
    VARIABLE_PROCESSOR.set_global_variable(name, value);
    Ok(())
}

// Tauriコマンド: グローバル変数を取得
#[tauri::command]
fn get_global_variables() -> Result<HashMap<String, String>, String> {
    Ok(VARIABLE_PROCESSOR.get_all_global_variables())
}

// Tauriコマンド: YAMLから変数を読み込み
#[tauri::command]
fn load_variables_from_yaml(yaml_content: String) -> Result<(), String> {
    VARIABLE_PROCESSOR
        .load_variables_from_yaml(&yaml_content)
        .map_err(|e| e.to_string())
}

// Tauriコマンド: 変数をYAML形式でエクスポート
#[tauri::command]
fn export_variables_to_yaml() -> Result<String, String> {
    VARIABLE_PROCESSOR
        .export_variables_to_yaml()
        .map_err(|e| e.to_string())
}

// Tauriコマンド: Markdownを処理（変数展開）
#[tauri::command]
fn process_markdown(
    content: String,
    global_variables: HashMap<String, String>,
) -> Result<String, String> {
    // グローバル変数を一時的に設定
    for (name, value) in global_variables {
        VARIABLE_PROCESSOR.set_global_variable(name, value);
    }

    let result = VARIABLE_PROCESSOR.process_variables(&content);
    Ok(result)
}

// Tauriコマンド: 変数展開済みのMarkdownコンテンツを取得
#[tauri::command]
fn get_expanded_markdown(
    content: String,
    global_variables: HashMap<String, String>,
) -> Result<String, String> {
    // グローバル変数を一時的に設定
    for (name, value) in global_variables {
        VARIABLE_PROCESSOR.set_global_variable(name, value);
    }

    let result = VARIABLE_PROCESSOR.process_variables(&content);
    Ok(result)
}

// Tauriコマンド: ファイルを読み込み
#[tauri::command]
async fn read_file(path: String) -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    // ファイルサイズチェック（10MB制限）
    let metadata = fs::metadata(&path).map_err(|_| "File not found".to_string())?;
    if metadata.len() > 10 * 1024 * 1024 {
        return Err("File too large (max 10MB)".to_string());
    }

    // ファイル拡張子チェック
    if let Some(ext) = Path::new(&path).extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str != "md" && ext_str != "txt" {
            return Err("Unsupported file type. Only .md and .txt files are supported".to_string());
        }
    }

    // ファイル読み込み
    fs::read_to_string(&path).map_err(|_| "Failed to read file".to_string())
}

// Tauriコマンド: ファイルを保存
#[tauri::command]
async fn save_file(path: String, content: String) -> Result<(), String> {
    use std::fs;
    use std::path::Path;

    // ファイル拡張子チェック
    if let Some(ext) = Path::new(&path).extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str != "md" && ext_str != "txt" {
            return Err("Unsupported file type. Only .md and .txt files are supported".to_string());
        }
    }

    // ディレクトリ作成
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|_| "Failed to create directory".to_string())?;
    }

    // ファイル保存
    fs::write(&path, content).map_err(|_| "Failed to save file".to_string())
}

// Tauriコマンド: ウィンドウ情報を取得
#[tauri::command]
async fn get_window_info(window: Window) -> Result<WindowInfo, String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
    let is_minimized = window.is_minimized().map_err(|e| e.to_string())?;

    Ok(WindowInfo {
        width: size.width as f64,
        height: size.height as f64,
        x: position.x as f64,
        y: position.y as f64,
        maximized: is_maximized,
        minimized: is_minimized,
    })
}

// Tauriコマンド: ウィンドウ情報を設定
#[tauri::command]
async fn set_window_info(window: Window, info: WindowInfo) -> Result<(), String> {
    // 最小化状態を解除
    if info.minimized {
        window.unminimize().map_err(|e| e.to_string())?;
    }

    // 最大化状態を設定
    if info.maximized {
        window.maximize().map_err(|e| e.to_string())?;
    } else {
        // 通常サイズに設定
        window
            .set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                info.width,
                info.height,
            )))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
                info.x, info.y,
            )))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// Tauriコマンド: システム情報を取得
#[tauri::command]
async fn get_system_info(window: Window) -> Result<SystemInfo, String> {
    let app_handle = window.app_handle();

    // 全モニターを取得
    let monitors = match app_handle.available_monitors() {
        Ok(monitors) => monitors,
        Err(e) => return Err(format!("Failed to get monitors: {}", e)),
    };

    if monitors.is_empty() {
        return Err("No monitors found".to_string());
    }

    // プライマリモニターを取得
    let primary_monitor = match app_handle.primary_monitor() {
        Ok(Some(monitor)) => monitor,
        Ok(None) => return Err("No primary monitor found".to_string()),
        Err(e) => return Err(format!("Failed to get primary monitor: {}", e)),
    };

    let mut displays = Vec::new();
    let mut primary_display_index = 0;

    // プライマリモニターの情報を取得
    let primary_size = primary_monitor.size();
    let primary_position = primary_monitor.position();
    let primary_scale_factor = primary_monitor.scale_factor();

    // 全モニターの情報を収集
    for (index, monitor) in monitors.iter().enumerate() {
        let size = monitor.size();
        let position = monitor.position();
        let scale_factor = monitor.scale_factor();

        // 値の妥当性をチェック
        if size.width == 0 || size.height == 0 {
            println!(
                "Warning: Skipping monitor with invalid size: {}x{}",
                size.width, size.height
            );
            continue;
        }

        // プライマリモニターの判定（位置とサイズで比較）
        let is_primary = size.width == primary_size.width
            && size.height == primary_size.height
            && position.x == primary_position.x
            && position.y == primary_position.y;

        if is_primary {
            primary_display_index = index;
        }

        let display_info = DisplayInfo {
            width: size.width as f64,
            height: size.height as f64,
            x: position.x as f64,
            y: position.y as f64,
            scale_factor,
            is_primary,
        };

        displays.push(display_info);

        println!(
            "Debug: Monitor {} - size: {}x{}, pos: ({}, {}), scale: {}, primary: {}",
            index, size.width, size.height, position.x, position.y, scale_factor, is_primary
        );
    }

    if displays.is_empty() {
        return Err("No valid monitors found".to_string());
    }

    let system_info = SystemInfo {
        displays,
        primary_display_index,
    };

    println!(
        "Debug: System info created - displays: {}, primary_index: {}",
        system_info.displays.len(),
        system_info.primary_display_index
    );

    Ok(system_info)
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            set_global_variable,
            get_global_variables,
            load_variables_from_yaml,
            export_variables_to_yaml,
            process_markdown,
            get_expanded_markdown,
            read_file,
            save_file,
            get_window_info,
            set_window_info,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

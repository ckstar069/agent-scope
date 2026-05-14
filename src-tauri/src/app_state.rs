use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::collectors::agent::AgentCollector;
use crate::collectors::template::{
    load_template_path, TemplateFingerprint,
};
use crate::registry::ProjectRegistry;

/// 模板指纹缓存 — 记录模板目录中所有文件路径的快照
#[derive(Debug, Clone)]
pub struct TemplateFingerprintCache {
    pub paths: std::collections::HashSet<String>,
    pub generated_at: std::time::Instant,
}

pub struct AppState {
    pub registry: Mutex<ProjectRegistry>,
    pub watchers: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_collector: Mutex<AgentCollector>,
    pub template_path: Mutex<Option<PathBuf>>,
    pub template_fingerprint: Mutex<Option<TemplateFingerprintCache>>,
}

impl AppState {
    pub fn new(registry: ProjectRegistry, agent_collector: AgentCollector) -> Self {
        Self {
            registry: Mutex::new(registry),
            watchers: Mutex::new(HashMap::new()),
            agent_collector: Mutex::new(agent_collector),
            template_path: Mutex::new(None),
            template_fingerprint: Mutex::new(None),
        }
    }
}

pub fn init_app_state(
    app: &tauri::App,
    registry: ProjectRegistry,
    agent_collector: AgentCollector,
) {
    let state = AppState::new(registry, agent_collector);

    let data_dir = ProjectRegistry::default_data_dir();
    if let Some(template_path) = load_template_path(&data_dir) {
        if template_path.exists() && template_path.is_dir() {
            if let Ok(fingerprint) = TemplateFingerprint::build(&template_path) {
                if let Ok(mut tp) = state.template_path.lock() {
                    *tp = Some(template_path);
                }
                if let Ok(mut fp) = state.template_fingerprint.lock() {
                    *fp = Some(TemplateFingerprintCache {
                        paths: fingerprint.paths,
                        generated_at: std::time::Instant::now(),
                    });
                }
            }
        } else {
            eprintln!(
                "[init_app_state] 警告: 已保存的模板路径不存在或不是目录: {}",
                template_path.display()
            );
        }
    }

    app.manage(state);
}

use crate::sensevoice::bundled_paths;

#[derive(Clone)]
pub enum HealthStatus {
    Ok,
    Warning { message: String, fix_hint: String },
}

impl serde::Serialize for HealthStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            HealthStatus::Ok => {
                map.serialize_entry("status", "ok")?;
            }
            HealthStatus::Warning { message, fix_hint } => {
                map.serialize_entry("status", "warning")?;
                map.serialize_entry("message", message)?;
                map.serialize_entry("fix_hint", fix_hint)?;
            }
        }
        map.end()
    }
}

#[derive(Clone)]
pub struct HealthCheckResult {
    pub id: String,
    pub name: String,
    pub status: HealthStatus,
}

impl serde::Serialize for HealthCheckResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("status", &self.status)?;
        map.end()
    }
}

pub struct HealthState {
    pub checks: Vec<HealthCheckResult>,
}

pub trait HealthCheck {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn check(&self, app: &tauri::AppHandle) -> HealthStatus;
}

pub struct OnnxRuntimeCheck;

impl HealthCheck for OnnxRuntimeCheck {
    fn id(&self) -> &str {
        "onnx_runtime"
    }

    fn name(&self) -> &str {
        "ONNX Runtime"
    }

    fn check(&self, app: &tauri::AppHandle) -> HealthStatus {
        match bundled_paths::onnxruntime_dylib_path(app) {
            Some(_) => HealthStatus::Ok,
            None => HealthStatus::Warning {
                message: "ONNX Runtime 动态库未找到，SenseVoice 本地转写功能将不可用。".to_string(),
                fix_hint: "请通过 brew install onnxruntime 安装。".to_string(),
            },
        }
    }
}

pub fn run_health_checks(app: &tauri::AppHandle) -> Vec<HealthCheckResult> {
    let checks: Vec<Box<dyn HealthCheck>> = vec![Box::new(OnnxRuntimeCheck)];
    checks
        .iter()
        .map(|c| HealthCheckResult {
            id: c.id().to_string(),
            name: c.name().to_string(),
            status: c.check(app),
        })
        .collect()
}

#[tauri::command]
pub fn get_health_status(state: tauri::State<HealthState>) -> Vec<HealthCheckResult> {
    state.checks.clone()
}
